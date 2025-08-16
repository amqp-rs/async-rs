<div align="center">

[![API Docs](https://docs.rs/async-rs/badge.svg)](https://docs.rs/async-rs)
[![Build status](https://github.com/amqp-rs/async-rs/workflows/Build%20and%20test/badge.svg)](https://github.com/amqp-rs/async-rs/actions)
[![Downloads](https://img.shields.io/crates/d/async-rs.svg)](https://crates.io/crates/async-rs)
[![Dependency Status](https://deps.rs/repo/github/amqp-rs/async-rs/status.svg)](https://deps.rs/repo/github/amqp-rs/async-rs)
[![LICENSE](https://img.shields.io/github/license/amqp-rs/async-rs)](LICENSE)

 <strong>
   A Rust async runtime abstration library.
 </strong>

</div>

<br />

## Features

- tokio: enable the tokio implementation *(default)*
- smol: enable the smol implementation
- async-global-executor: enable the async-global-executor implementation
- async-io: enable the async-io reactor implementation

## Example

```rust
use async_rs::{Executor, Reactor, Runtime, TokioRuntime};
use std::{io, sync::Arc, time::Duration};

async fn get_a(rt: Arc<TokioRuntime>) -> io::Result<u32> {
    rt.clone()
        .spawn_blocking(move || rt.block_on(async { Ok(12) }))
        .await
}

async fn get_b(rt: Arc<TokioRuntime>) -> io::Result<u32> {
    rt.spawn(async { Ok(30) }).await
}

async fn tokio_main() -> io::Result<()> {
    let rt = Arc::new(Runtime::tokio());
    let a = get_a(rt.clone()).await?;
    let b = get_b(rt.clone()).await?;
    rt.sleep(Duration::from_millis(500)).await;
    assert_eq!(a + b, 42);
    Ok(())
}

#[tokio::main]
async fn main() -> io::Result<()> {
    tokio_main().await
}
```
