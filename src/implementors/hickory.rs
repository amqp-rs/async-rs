use crate::{traits::AsyncToSocketAddrs, util::SocketAddrsFromIpAddrs};
use hickory_resolver::{IntoName, Resolver};
use std::{io, net::SocketAddr, str::FromStr};

/// Perform async DNS resolution using hickory-dns
#[derive(Debug)]
pub struct HickoryToSocketAddrs<T: IntoName + Send + 'static> {
    host: T,
    port: u16,
}

impl<H: IntoName + Send + 'static> HickoryToSocketAddrs<H> {
    /// Create a `HickoryToSocketAddrs` from split host and port components.
    pub fn new(host: H, port: u16) -> Self {
        Self { host, port }
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

impl<T: IntoName + Send + 'static> AsyncToSocketAddrs for HickoryToSocketAddrs<T> {
    fn to_socket_addrs(
        self,
    ) -> impl Future<Output = io::Result<impl Iterator<Item = SocketAddr> + Send + 'static>>
    + Send
    + 'static {
        async move {
            if tokio::runtime::Handle::try_current().is_err() {
                return Err(io::Error::other(
                    "hickory-dns is only supported in a tokio context",
                ));
            }

            Ok(SocketAddrsFromIpAddrs(
                Resolver::builder_tokio()?
                    .build()
                    .lookup_ip(self.host)
                    .await?
                    .into_iter(),
                self.port,
            ))
        }
    }
}
