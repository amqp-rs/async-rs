use crate::{
    Runtime,
    traits::{AsyncToSocketAddrs, Executor, RuntimeKit},
};
use std::{
    fmt, future, io,
    net::{IpAddr, SocketAddr, ToSocketAddrs},
};

/// Wrapper to impl AsyncToSocketAddrs from an IntoIterator<Item = SocketAddr>
pub struct SocketAddrs<I: IntoIterator<Item = SocketAddr> + Send + 'static>(pub I);

impl<I: IntoIterator<Item = SocketAddr> + Send + fmt::Debug + 'static> AsyncToSocketAddrs
    for SocketAddrs<I>
where
    I::IntoIter: Send + 'static,
{
    fn to_socket_addrs(
        self,
    ) -> impl Future<Output = io::Result<impl Iterator<Item = SocketAddr> + Send + 'static>>
    + Send
    + 'static {
        future::ready(Ok(self.0.into_iter()))
    }
}

impl<I: IntoIterator<Item = SocketAddr> + Send + fmt::Debug + 'static> fmt::Debug
    for SocketAddrs<I>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("SocketAddrs").field(&self.0).finish()
    }
}

/// Iterator for SocketAddr computed from IpAddr + port
pub struct SocketAddrsFromIpAddrs<I: Iterator<Item = IpAddr> + Send + 'static>(pub I, pub u16);

impl<I: Iterator<Item = IpAddr> + Send + 'static> Iterator for SocketAddrsFromIpAddrs<I> {
    type Item = SocketAddr;

    fn next(&mut self) -> Option<Self::Item> {
        Some(SocketAddr::new(self.0.next()?, self.1))
    }
}

impl<I: Iterator<Item = IpAddr> + Send + 'static> fmt::Debug for SocketAddrsFromIpAddrs<I> {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_tuple("SocketAddrsFromIpAddrs").finish()
    }
}

/// Wrapper to perform blocking name resolution on top of an async runtime
pub struct SocketAddrsResolver<'a, RK: RuntimeKit, A: ToSocketAddrs + Send + 'static> {
    pub(crate) runtime: &'a Runtime<RK>,
    pub(crate) addrs: A,
}

impl<'a, RK: RuntimeKit, A: ToSocketAddrs + Send + 'static> AsyncToSocketAddrs
    for SocketAddrsResolver<'a, RK, A>
where
    <A as ToSocketAddrs>::Iter: Iterator<Item = SocketAddr> + Send + 'static,
{
    fn to_socket_addrs(
        self,
    ) -> impl Future<Output = io::Result<impl Iterator<Item = SocketAddr> + Send + 'static>>
    + Send
    + 'static {
        let SocketAddrsResolver { runtime, addrs } = self;
        runtime.spawn_blocking(move || addrs.to_socket_addrs())
    }
}

impl<'a, RK: RuntimeKit, A: ToSocketAddrs + Send + fmt::Debug + 'static> fmt::Debug
    for SocketAddrsResolver<'a, RK, A>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SocketAddrsResolver")
            .field("addrs", &self.addrs)
            .finish()
    }
}
