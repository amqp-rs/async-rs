use crate::{
    Runtime,
    traits::{AsyncToSocketAddrs, Executor, RuntimeKit},
};
use std::{
    fmt, io,
    net::{SocketAddr, ToSocketAddrs},
};

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
