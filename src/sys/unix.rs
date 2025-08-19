use std::os::unix::io::{AsFd, AsRawFd};

/// Abstract trait on top of AsFd + AsRawFd or AsSocket + AsRawSocket for unix or windows
pub trait AsSysFd: AsFd + AsRawFd {}
impl<H: AsFd + AsRawFd> AsSysFd for H {}

#[cfg(feature = "async-io")]
mod async_io {
    use crate::{sys::AsSysFd, util::IOHandle};
    use std::{
        io::{Read, Write},
        os::unix::io::{AsFd, AsRawFd, BorrowedFd, RawFd},
    };

    impl<H: Read + Write + AsSysFd + Send + 'static> AsFd for IOHandle<H> {
        fn as_fd(&self) -> BorrowedFd<'_> {
            self.0.as_fd()
        }
    }

    impl<H: Read + Write + AsSysFd + Send + 'static> AsRawFd for IOHandle<H> {
        fn as_raw_fd(&self) -> RawFd {
            self.as_fd().as_raw_fd()
        }
    }
}
