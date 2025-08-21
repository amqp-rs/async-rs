use crate::{traits::AsyncToSocketAddrs, util::SocketAddrsFromIpAddrs};
use hickory_resolver::{IntoName, Resolver};
use std::{io, net::SocketAddr};

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
