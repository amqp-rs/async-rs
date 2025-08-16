use crate::sys::IO;
use std::{
    fmt,
    io::{self, IoSlice, IoSliceMut, Read, Write},
};

/// A synchronous IO handle
pub struct IOHandle<H: IO + Send + 'static>(pub(crate) H);

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

#[allow(unsafe_code)]
#[cfg(feature = "async-io")]
unsafe impl<H: IO + Send + 'static> async_io::IoSafe for IOHandle<H> {}
