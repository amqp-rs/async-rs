use std::{
    future, io,
    net::{IpAddr, SocketAddr},
};

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

impl<A: Into<SocketAddr> + sealed::SocketSealed> AsyncToSocketAddrs for A {
    fn to_socket_addrs(
        self,
    ) -> impl Future<Output = io::Result<impl Iterator<Item = SocketAddr> + Send + 'static>>
    + Send
    + 'static {
        future::ready(Ok(Some(self.into()).into_iter()))
    }
}

impl<I: Into<IpAddr> + sealed::IpSealed> AsyncToSocketAddrs for (I, u16) {
    fn to_socket_addrs(
        self,
    ) -> impl Future<Output = io::Result<impl Iterator<Item = SocketAddr> + Send + 'static>>
    + Send
    + 'static {
        future::ready(Ok(Some((self.0.into(), self.1).into()).into_iter()))
    }
}

impl AsyncToSocketAddrs for Vec<SocketAddr> {
    fn to_socket_addrs(
        self,
    ) -> impl Future<Output = io::Result<impl Iterator<Item = SocketAddr> + Send + 'static>>
    + Send
    + 'static {
        future::ready(Ok(self.into_iter()))
    }
}

impl AsyncToSocketAddrs for &[SocketAddr] {
    fn to_socket_addrs(
        self,
    ) -> impl Future<Output = io::Result<impl Iterator<Item = SocketAddr> + Send + 'static>>
    + Send
    + 'static {
        self.to_vec().to_socket_addrs()
    }
}

mod sealed {
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6};

    pub trait SocketSealed {}

    // Into<SocketAddr>
    impl SocketSealed for SocketAddr {}
    impl SocketSealed for SocketAddrV4 {}
    impl SocketSealed for SocketAddrV6 {}

    pub trait IpSealed {}

    // Into<IpAddr>
    impl IpSealed for IpAddr {}
    impl IpSealed for Ipv4Addr {}
    impl IpSealed for Ipv6Addr {}
    impl IpSealed for [u8; 4] {}
    impl IpSealed for [u8; 16] {}
    impl IpSealed for [u16; 8] {}
}
