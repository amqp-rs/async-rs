use crate::util::IOHandle;
use std::{
    io::{Read, Write},
    os::unix::io::{AsFd, AsRawFd, BorrowedFd, RawFd},
};

/// Abstract trait on top of Read + Write + AsFd or AsSocket for unix or windows
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
