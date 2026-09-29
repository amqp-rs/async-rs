#[cfg(any(
    feature = "tokio",
    feature = "smol",
    all(feature = "async-global-executor", feature = "async-io")
))]
use async_rs::traits::Executor;
#[cfg(any(feature = "tokio", feature = "smol", feature = "async-io"))]
use async_rs::traits::Reactor;
#[cfg(any(feature = "tokio", feature = "smol", feature = "async-io"))]
use futures_core::Stream;
#[cfg(any(
    feature = "tokio",
    feature = "smol",
    all(feature = "async-global-executor", feature = "async-io")
))]
use std::future::Future;
#[cfg(feature = "async-io")]
use std::task::{Context, Waker};
#[cfg(any(feature = "tokio", feature = "smol", feature = "async-io"))]
use std::{task::Poll, time::Duration};

#[cfg(feature = "async-io")]
fn first_interval_tick_is_ready<R: Reactor>(reactor: &R) {
    let mut ticks = Box::pin(reactor.interval(Duration::from_secs(30)));
    let mut cx = Context::from_waker(Waker::noop());
    assert!(matches!(
        ticks.as_mut().poll_next(&mut cx),
        Poll::Ready(Some(_))
    ));
}

#[cfg(any(
    feature = "tokio",
    feature = "smol",
    all(feature = "async-global-executor", feature = "async-io")
))]
fn task_and_timer_contract<R: Executor + Reactor>(runtime: &R) {
    assert_eq!(runtime.block_on(runtime.spawn(async { 42 })), 42);
    assert_eq!(runtime.block_on(runtime.spawn_blocking(|| 43)), 43);

    let mut pending = runtime.spawn(std::future::pending::<()>());
    assert_eq!(runtime.block_on(pending.cancel()), None);

    let (tx, rx) = std::sync::mpsc::channel();
    drop(runtime.spawn(async move { tx.send(()).unwrap() }));
    rx.recv_timeout(Duration::from_secs(5)).unwrap();

    let start = std::time::Instant::now();
    runtime.block_on(runtime.sleep(Duration::from_millis(10)));
    assert!(start.elapsed() >= Duration::from_millis(5));

    let mut ticks = Box::pin(runtime.interval(Duration::from_secs(30)));
    let mut timeout = Box::pin(runtime.sleep(Duration::from_millis(200)));
    let first_tick_arrived = runtime.block_on(std::future::poll_fn(|cx| {
        if matches!(ticks.as_mut().poll_next(cx), Poll::Ready(Some(_))) {
            return Poll::Ready(true);
        }
        if timeout.as_mut().poll(cx).is_ready() {
            return Poll::Ready(false);
        }
        Poll::Pending
    }));
    assert!(first_tick_arrived);
}

#[cfg(feature = "tokio")]
#[test]
fn tokio_contract() {
    task_and_timer_contract(&async_rs::Runtime::tokio().unwrap());
}

#[cfg(feature = "smol")]
#[test]
fn smol_contract() {
    task_and_timer_contract(&async_rs::Runtime::smol());
}

#[cfg(all(feature = "async-global-executor", feature = "async-io"))]
#[test]
fn async_global_executor_contract() {
    task_and_timer_contract(&async_rs::Runtime::async_global_executor());
}

#[cfg(feature = "async-io")]
#[test]
fn async_io_interval_starts_immediately() {
    first_interval_tick_is_ready(&async_rs::AsyncIO);
}
