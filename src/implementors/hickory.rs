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

fn get_or_init_resolver() -> io::Result<&'static TokioResolver> {
    // FIXME: replace with RESOLVER.get_or_try_init(...) once it stabilises (rust#109737)
    if let Some(r) = RESOLVER.get() {
        return Ok(r);
    }
    let resolver = TokioResolver::builder_tokio()
        .map_err(io::Error::other)?
        .build()
        .map_err(io::Error::other)?;
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
    pub fn new(host: H, port: u16) -> Self {
        Self { host, port }
    }

    async fn lookup(self) -> io::Result<SocketAddrsFromIpAddrs<vec::IntoIter<IpAddr>>> {
        if !util::inside_tokio() {
            return Err(io::Error::other(
                "hickory-dns is only supported in a tokio context",
            ));
        }

        let resolver = get_or_init_resolver()?;

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
        let (host, port_str) = s
            .rsplit_once(':')
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "invalid socket address"))?;
        let port = port_str
            .parse()
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid port value"))?;
        Ok(Self::new(host.to_owned(), port))
    }
}

impl<T: IntoName + Clone + Send + 'static> ToSocketAddrs for HickoryToSocketAddrs<T> {
    type Iter = SocketAddrsFromIpAddrs<vec::IntoIter<IpAddr>>;

    fn to_socket_addrs(&self) -> io::Result<Self::Iter> {
        util::block_on_tokio(self.clone().lookup())
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
