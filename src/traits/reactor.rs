//! A collection of traits to define a common interface across reactors

use async_trait::async_trait;
use futures_core::Stream;
use futures_io::{AsyncRead, AsyncWrite};
use std::{
    fmt,
    io::{self, IoSlice, IoSliceMut, Read, Write},
    net::SocketAddr,
    time::{Duration, Instant},
};
use sys::IO;

/// A common interface for performing actions on a reactor
#[async_trait]
pub trait Reactor: fmt::Debug {
    /// Register a synchronous handle, returning an asynchronous one
    fn register<H: IO + Send + 'static>(
        &self,
        socket: IOHandle<H>,
    ) -> io::Result<impl AsyncIOHandle + Send>
    where
        Self: Sized;

    /// Sleep for the given duration
    async fn sleep(&self, dur: Duration);

    /// Stream that yields at every given interval
    fn interval(&self, dur: Duration) -> impl Stream<Item = Instant>
    where
        Self: Sized;

    /// Create a TcpStream by connecting to a remote host
    async fn tcp_connect(&self, addr: SocketAddr) -> io::Result<impl AsyncIOHandle + Send>
    where
        Self: Sized;
}

/// A synchronous IO handle
pub struct IOHandle<H: IO + Send + 'static>(H);

impl<H: IO + Send + 'static> IOHandle<H> {
    /// Instantiate a new IO handle
    pub fn new(io: H) -> Self {
        Self(io)
    }
}

impl<H: IO + Send + 'static> Read for IOHandle<H> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.read(buf)
    }

    fn read_vectored(&mut self, bufs: &mut [IoSliceMut<'_>]) -> io::Result<usize> {
        self.0.read_vectored(bufs)
    }

    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> io::Result<usize> {
        self.0.read_to_end(buf)
    }

    fn read_to_string(&mut self, buf: &mut String) -> io::Result<usize> {
        self.0.read_to_string(buf)
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> io::Result<()> {
        self.0.read_exact(buf)
    }
}

impl<H: IO + Send + 'static> Write for IOHandle<H> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }

    fn write_vectored(&mut self, bufs: &[IoSlice<'_>]) -> io::Result<usize> {
        self.0.write_vectored(bufs)
    }

    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        self.0.write_all(buf)
    }

    fn write_fmt(&mut self, fmt: fmt::Arguments<'_>) -> io::Result<()> {
        self.0.write_fmt(fmt)
    }
}

impl<H: IO + Send + 'static> fmt::Debug for IOHandle<H> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("IOHandle").finish()
    }
}

/// A trait representing an asynchronous IO handle
pub trait AsyncIOHandle: AsyncRead + AsyncWrite {}
impl<H: AsyncRead + AsyncWrite> AsyncIOHandle for H {}

#[cfg(unix)]
pub(crate) mod sys {
    use crate::IOHandle;
    use std::{
        io::{Read, Write},
        os::unix::io::{AsFd, AsRawFd, BorrowedFd, RawFd},
    };

    pub trait IO: Read + Write + AsFd {}
    impl<H: Read + Write + AsFd> IO for H {}

    impl<H: IO + Send + 'static> AsFd for IOHandle<H> {
        fn as_fd(&self) -> BorrowedFd<'_> {
            self.0.as_fd()
        }
    }

    impl<H: IO + Send + 'static> AsRawFd for IOHandle<H> {
        fn as_raw_fd(&self) -> RawFd {
            self.as_fd().as_raw_fd()
        }
    }
}

#[cfg(windows)]
pub(crate) mod sys {
    use crate::IOHandle;
    use std::{
        io::{Read, Write},
        os::windows::io::{AsRawSocket, AsSocket, BorrowedSocket, RawSocket},
    };

    pub trait IO: Read + Write + AsSocket {}
    impl<H: Read + Write + AsSocket> IO for H {}

    impl<H: IO + Send + 'static> AsSocket for IOHandle<H> {
        fn as_socket(&self) -> BorrowedSocket<'_> {
            self.0.as_socket()
        }
    }

    impl<H: IO + Send + 'static> AsRawSocket for IOHandle<H> {
        fn as_raw_socket(&self) -> RawSocket {
            self.as_socket().as_raw_socket()
        }
    }
}
