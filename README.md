<div align="center">

[![API Docs](https://docs.rs/async-rs/badge.svg)](https://docs.rs/async-rs)
[![Build status](https://github.com/amqp-rs/async-rs/workflows/Build%20and%20test/badge.svg)](https://github.com/amqp-rs/async-rs/actions)
[![Downloads](https://img.shields.io/crates/d/async-rs.svg)](https://crates.io/crates/async-rs)
[![Dependency Status](https://deps.rs/repo/github/amqp-rs/async-rs/status.svg)](https://deps.rs/repo/github/amqp-rs/async-rs)
[![LICENSE](https://img.shields.io/crates/l/async-rs)](LICENSE)

**A Rust async runtime abstraction library.**

</div>

Provides a unified `Runtime` enum and a `traits` module (`Executor`, `Reactor`,
`Dns`, …) that abstract over Tokio, smol, and async-global-executor.
Applications select exactly one runtime via feature flags; library crates
depend on the trait objects and remain runtime-agnostic.

## Feature flags

| Flag | Notes |
|------|-------|
| `tokio` *(default)* | Tokio runtime |
| `smol` | smol executor |
| `async-global-executor` | async-global-executor |
| `async-io` | async-io reactor (required by `smol`) |
| `hickory-dns` | Hickory DNS resolver (tokio only) |

## Example

```rust
use async_rs::{Runtime, TokioRuntime, traits::*};
use std::{io, time::Duration};

async fn get_a(rt: &TokioRuntime) -> io::Result<u32> {
    rt.spawn_blocking(|| Ok(12)).await
}

async fn get_b(rt: &TokioRuntime) -> io::Result<u32> {
    rt.spawn(async { Ok(30) }).await
}

async fn tokio_main(rt: &TokioRuntime) -> io::Result<()> {
    let a = get_a(rt).await?;
    let b = get_b(rt).await?;
    rt.sleep(Duration::from_millis(500)).await;
    assert_eq!(a + b, 42);
    Ok(())
}

fn main() -> io::Result<()> {
    let rt = Runtime::tokio()?;
    rt.block_on(tokio_main(&rt))
}
```
