use std::{io, net::SocketAddr};

/// A common interface for resolving domain name + port to `SocketAddr`
pub trait AsyncToSocketAddrs {
    /// Resolve the domain name through DNS and return an `Iterator` of `SocketAddr`
    fn to_socket_addrs(
        self,
    ) -> impl Future<Output = io::Result<impl Iterator<Item = SocketAddr> + Send + 'static>>
    + Send
    + 'static
    where
        Self: Sized;
}
