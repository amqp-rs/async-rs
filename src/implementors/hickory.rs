use crate::traits::AsyncToSocketAddrs;
use hickory_resolver::{IntoName, Resolver, lookup_ip::LookupIpIntoIter};
use std::{fmt, io, net::SocketAddr};

/// Perform async DNS resolution using hickory-dns
#[derive(Debug)]
pub struct HickoryToSocketAddrs<T: IntoName + Send + 'static> {
    host: T,
    port: u16,
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

            Ok(HickorySocketAddrs(
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

/// Iterator for SocketAddr resolved by `hickory-dns`
pub struct HickorySocketAddrs(LookupIpIntoIter, u16);

impl Iterator for HickorySocketAddrs {
    type Item = SocketAddr;

    fn next(&mut self) -> Option<Self::Item> {
        Some(SocketAddr::new(self.0.next()?, self.1))
    }
}

impl fmt::Debug for HickorySocketAddrs {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_tuple("HickorySocketAddrs").finish()
    }
}
