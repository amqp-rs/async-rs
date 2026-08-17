use crate::{
    traits::AsyncToSocketAddrs,
    util::{self, SocketAddrsFromIpAddrs},
};
use hickory_resolver::{TokioResolver, proto::rr::IntoName};
use std::{
    io,
    net::{IpAddr, SocketAddr, ToSocketAddrs},
    str::FromStr,
    sync::OnceLock,
    vec,
};

static RESOLVER: OnceLock<TokioResolver> = OnceLock::new();

/// Build a resolver, which binds to the tokio runtime it is used from as it opens connections.
fn new_resolver() -> io::Result<TokioResolver> {
    TokioResolver::builder_tokio()
        .map_err(io::Error::other)?
        .build()
        .map_err(io::Error::other)
}

/// The shared resolver, for lookups running on a runtime the caller brought and keeps.
///
/// Only ever initialised from such a lookup: a resolver caches its name-server connections, so one
/// built on a runtime which is about to go away would poison this for every later caller.
fn get_or_init_resolver() -> io::Result<&'static TokioResolver> {
    // FIXME: replace with RESOLVER.get_or_try_init(...) once it stabilises (rust#109737)
    if let Some(r) = RESOLVER.get() {
        return Ok(r);
    }
    let resolver = new_resolver()?;
    Ok(RESOLVER.get_or_init(|| resolver))
}

/// Perform async DNS resolution using hickory-dns
#[derive(Debug, Clone)]
pub struct HickoryToSocketAddrs<T: IntoName + Send + 'static> {
    host: T,
    port: u16,
}

impl<H: IntoName + Send + 'static> HickoryToSocketAddrs<H> {
    /// Create a `HickoryToSocketAddrs` from split host and port components.
    ///
    /// The host is passed to the resolver as given. An IP literal resolves without a query, but
    /// only in the form `IpAddr` itself parses, so hand over `::1` rather than `[::1]` — the
    /// bracketed form is a socket-address spelling and would go out as a hostname. Parsing a
    /// whole `host:port` string with [`FromStr`] unwraps the brackets for you.
    pub fn new(host: H, port: u16) -> Self {
        Self { host, port }
    }

    async fn lookup(self) -> io::Result<SocketAddrsFromIpAddrs<vec::IntoIter<IpAddr>>> {
        if !util::inside_tokio() {
            return Err(io::Error::other(
                "hickory-dns is only supported in a tokio context",
            ));
        }

        self.lookup_with(get_or_init_resolver()?).await
    }

    async fn lookup_with(
        self,
        resolver: &TokioResolver,
    ) -> io::Result<SocketAddrsFromIpAddrs<vec::IntoIter<IpAddr>>> {
        Ok(SocketAddrsFromIpAddrs(
            resolver
                .lookup_ip(self.host)
                .await
                .map_err(io::Error::other)?
                .iter()
                .collect::<Vec<_>>() // FIXME: don't collect if we get back into_iter
                .into_iter(),
            self.port,
        ))
    }
}

impl FromStr for HickoryToSocketAddrs<String> {
    type Err = io::Error;

    fn from_str(s: &str) -> io::Result<Self> {
        fn invalid(msg: &'static str) -> io::Error {
            io::Error::new(io::ErrorKind::InvalidInput, msg)
        }

        // hickory shortcuts a host which parses as an IP address and answers without a query, but
        // only if we hand it the bare address, so let std unwrap the brackets an IPv6 literal
        // comes in. Brackets are reserved for that spelling, so anything else in them is malformed
        // rather than a hostname we should quietly go and resolve.
        if let Ok(addr) = s.parse::<SocketAddr>() {
            // A numeric zone id survives `SocketAddr` but not the trip back out through `IpAddr`,
            // and there is nowhere to put one in a host and a port anyway. Dropping it silently
            // would connect to a link-local address over whichever interface the kernel picks.
            if matches!(addr, SocketAddr::V6(addr) if addr.scope_id() != 0) {
                return Err(invalid("IPv6 scope ids are not supported"));
            }
            return Ok(Self::new(addr.ip().to_string(), addr.port()));
        }
        if s.starts_with('[') {
            return Err(invalid("bracketed host is not an IP address"));
        }
        let (host, port_str) = s
            .rsplit_once(':')
            .ok_or_else(|| invalid("invalid socket address"))?;
        // The empty name is the DNS root, which resolves to no address at all, so a lookup for it
        // is a round trip whose only possible outcome is a misleading "couldn't resolve host".
        if host.is_empty() {
            return Err(invalid("empty host"));
        }
        // An unbracketed IPv6 literal keeps its own colons, so the split above would hand us its
        // last hextet as the port. Bracket it if it is meant to carry one.
        if host.contains(':') {
            return Err(invalid("IPv6 literals must be bracketed"));
        }
        let port = port_str
            .parse()
            .map_err(|_| invalid("invalid port value"))?;
        Ok(Self::new(host.to_owned(), port))
    }
}

impl<T: IntoName + Clone + Send + 'static> ToSocketAddrs for HickoryToSocketAddrs<T> {
    type Iter = SocketAddrsFromIpAddrs<vec::IntoIter<IpAddr>>;

    fn to_socket_addrs(&self) -> io::Result<Self::Iter> {
        if util::inside_tokio() {
            return util::block_on_tokio(self.clone().lookup());
        }
        // Off a tokio thread, `block_on_tokio` builds a runtime which dies with this call, so the
        // resolver has to die with it: caching one whose name-server connections are registered on
        // a driver that is gone leaves every later lookup, from anywhere, talking to a corpse.
        let this = self.clone();
        util::block_on_tokio(async move { this.lookup_with(&new_resolver()?).await })
    }
}

impl<T: IntoName + Send + 'static> AsyncToSocketAddrs for HickoryToSocketAddrs<T> {
    fn to_socket_addrs(
        self,
    ) -> impl Future<Output = io::Result<impl Iterator<Item = SocketAddr> + Send + 'static>>
    + Send
    + 'static {
        self.lookup()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(s: &str) -> (String, u16) {
        let addrs: HickoryToSocketAddrs<String> = s.parse().expect("parse");
        (addrs.host, addrs.port)
    }

    #[test]
    fn from_str_splits_host_and_port() {
        assert_eq!(parse("example.com:80"), ("example.com".to_owned(), 80));
    }

    #[test]
    fn from_str_keeps_ip_literals_parseable_as_ip() {
        // hickory only shortcuts these if IpAddr::from_str accepts what we hand it, which it does
        // not do for the bracketed form.
        for (input, host) in [("127.0.0.1:80", "127.0.0.1"), ("[::1]:80", "::1")] {
            let (parsed, port) = parse(input);
            assert_eq!((parsed.as_str(), port), (host, 80));
            assert!(parsed.parse::<IpAddr>().is_ok(), "{input}");
        }
    }

    #[test]
    fn from_str_rejects_garbage() {
        for input in [
            "example.com",
            "example.com:http",
            // An unbracketed IPv6 literal: splitting on the last colon would take `1` for a port
            // and leave `2001:db8:` as the host, which resolves to nothing.
            "2001:db8::1",
            "::1",
            // Brackets are the IP-literal spelling, so a hostname in them is malformed. Unwrapping
            // it would resolve a host the caller never asked for.
            "[example.com]:80",
            // A zone id does not survive `IpAddr::from_str`, so this would go out as a query for a
            // bogus name rather than shortcut. Both spellings, because std draws the line between
            // them and we do not: it rejects the named form outright, but parses the numeric one
            // and then loses the zone on the way back out to a string.
            "[fe80::1%eth0]:80",
            "[fe80::1%1]:80",
            // The empty host is the DNS root, not a hostname anybody meant to look up.
            ":80",
            // Half-bracketed input used to leak through as host `[::1`.
            "[::1:80",
            "[::1]",
            "[::1]80",
        ] {
            assert!(
                input.parse::<HickoryToSocketAddrs<String>>().is_err(),
                "{input}"
            );
        }
    }
}
