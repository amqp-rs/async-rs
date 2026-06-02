use futures_io::{AsyncRead, AsyncWrite};
use std::{
    fmt, io,
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll},
};

/// Split a bidirectional stream into independently-owned read and write halves.
///
/// Each half is `Send + 'static` (given the stream is), so the two can be driven by *separate*
/// futures/tasks — e.g. a reader loop and a writer loop running concurrently — without one holding
/// a `&mut` to the whole stream.
///
/// The halves share the stream behind a lock that is held only for the duration of a single
/// `poll_read`/`poll_write`/`poll_flush`/`poll_close`. Because reads and writes touch different
/// directions of the socket they never truly contend on data, only on this short critical section.
/// This is a runtime-agnostic split over the `AsyncRead + AsyncWrite` traits; a reactor that exposes
/// an owned, lock-free split of its own stream type can offer that separately.
pub fn split<S: AsyncRead + AsyncWrite + Unpin>(stream: S) -> (ReadHalf<S>, WriteHalf<S>) {
    let shared = Arc::new(Mutex::new(stream));
    (
        ReadHalf {
            shared: shared.clone(),
        },
        WriteHalf { shared },
    )
}

/// The read half produced by [`split`].
pub struct ReadHalf<S> {
    shared: Arc<Mutex<S>>,
}

/// The write half produced by [`split`].
pub struct WriteHalf<S> {
    shared: Arc<Mutex<S>>,
}

impl<S> fmt::Debug for ReadHalf<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ReadHalf").finish()
    }
}

impl<S> fmt::Debug for WriteHalf<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WriteHalf").finish()
    }
}

impl<S: AsyncRead + AsyncWrite + Unpin> ReadHalf<S> {
    /// Reunite the two halves back into the original stream, if they came from the same [`split`].
    #[must_use]
    pub fn unsplit(self, write: WriteHalf<S>) -> Option<S> {
        if Arc::ptr_eq(&self.shared, &write.shared) {
            drop(write);
            Arc::try_unwrap(self.shared)
                .ok()
                .map(|m| m.into_inner().unwrap_or_else(|e| e.into_inner()))
        } else {
            None
        }
    }
}

fn lock<S>(shared: &Arc<Mutex<S>>) -> std::sync::MutexGuard<'_, S> {
    shared.lock().unwrap_or_else(|e| e.into_inner())
}

impl<S: AsyncRead + Unpin> AsyncRead for ReadHalf<S> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut *lock(&self.shared)).poll_read(cx, buf)
    }

    fn poll_read_vectored(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bufs: &mut [io::IoSliceMut<'_>],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut *lock(&self.shared)).poll_read_vectored(cx, bufs)
    }
}

impl<S: AsyncWrite + Unpin> AsyncWrite for WriteHalf<S> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut *lock(&self.shared)).poll_write(cx, buf)
    }

    fn poll_write_vectored(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bufs: &[io::IoSlice<'_>],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut *lock(&self.shared)).poll_write_vectored(cx, bufs)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut *lock(&self.shared)).poll_flush(cx)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut *lock(&self.shared)).poll_close(cx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::simple_block_on;
    use std::{
        collections::VecDeque,
        future::poll_fn,
        pin::Pin,
        task::{Context, Poll},
    };

    // An in-memory duplex: writes append to an outgoing buffer, reads drain a preloaded incoming
    // buffer. Enough to exercise that the two halves poll independently.
    struct Duplex {
        incoming: VecDeque<u8>,
        outgoing: Vec<u8>,
    }

    impl AsyncRead for Duplex {
        fn poll_read(
            mut self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            buf: &mut [u8],
        ) -> Poll<io::Result<usize>> {
            let n = self.incoming.len().min(buf.len());
            for slot in buf.iter_mut().take(n) {
                *slot = self.incoming.pop_front().unwrap();
            }
            Poll::Ready(Ok(n))
        }
    }

    impl AsyncWrite for Duplex {
        fn poll_write(
            mut self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            buf: &[u8],
        ) -> Poll<io::Result<usize>> {
            self.outgoing.extend_from_slice(buf);
            Poll::Ready(Ok(buf.len()))
        }
        fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
            Poll::Ready(Ok(()))
        }
        fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
            Poll::Ready(Ok(()))
        }
    }

    #[test]
    fn split_read_and_write_independently() {
        let duplex = Duplex {
            incoming: b"hello".iter().copied().collect(),
            outgoing: Vec::new(),
        };
        let (mut r, mut w) = split(duplex);
        simple_block_on(async {
            let n = poll_fn(|cx| Pin::new(&mut w).poll_write(cx, b"world"))
                .await
                .unwrap();
            assert_eq!(n, 5);
            let mut buf = [0u8; 5];
            let n = poll_fn(|cx| Pin::new(&mut r).poll_read(cx, &mut buf))
                .await
                .unwrap();
            assert_eq!(n, 5);
            assert_eq!(&buf, b"hello");
        });
        let stream = r.unsplit(w).expect("same split");
        assert_eq!(stream.outgoing, b"world");
    }

    #[test]
    fn unsplit_rejects_foreign_half() {
        let (r1, _w1) = split(Duplex {
            incoming: VecDeque::new(),
            outgoing: Vec::new(),
        });
        let (_r2, w2) = split(Duplex {
            incoming: VecDeque::new(),
            outgoing: Vec::new(),
        });
        assert!(r1.unsplit(w2).is_none());
    }
}
