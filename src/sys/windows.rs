use crate::util::IOHandle;
use std::{
    io::{Read, Write},
    os::windows::io::{AsRawSocket, AsSocket, BorrowedSocket, RawSocket},
};

/// Abstract trait on top of Read + Write + AsFd or AsSocket for unix or windows
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
