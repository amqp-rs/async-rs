use std::os::windows::io::{AsRawSocket, AsSocket};

/// Abstract trait on top of AsFd + AsRawFd or AsSocket + AsRawSocket for unix or windows
pub trait AsSysFd: AsSocket + AsRawSocket {}
impl<H: AsSocket + AsRawSocket> AsSysFd for H {}

#[cfg(feature = "async-io")]
mod async_io {
    use crate::{sys::AsSysFd, util::IOHandle};
    use std::{
        io::{Read, Write},
        os::windows::io::{AsRawSocket, AsSocket, BorrowedSocket, RawSocket},
    };

    impl<H: Read + Write + AsSysFd + Send + 'static> AsSocket for IOHandle<H> {
        fn as_socket(&self) -> BorrowedSocket<'_> {
            self.0.as_socket()
        }
    }

    impl<H: Read + Write + AsSysFd + Send + 'static> AsRawSocket for IOHandle<H> {
        fn as_raw_socket(&self) -> RawSocket {
            self.0.as_raw_socket()
        }
    }
}
