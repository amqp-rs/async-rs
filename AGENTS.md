# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
# Build / check
cargo check --all --bins --examples --tests --all-features

# Test
cargo test

# Lint
cargo clippy --all-features -- -W clippy::all
cargo fmt --all -- --check
```

MSRV is **1.88.0** (Rust edition 2024).

## Architecture

`async-rs` is a thin abstraction layer that lets downstream crates (primarily the amqp-rs ecosystem) write runtime-agnostic async code. It exposes a unified `Runtime<RK>` wrapper that delegates to whichever async runtime is enabled via Cargo features.

### Trait hierarchy

```
RuntimeKit (supertrait)
├── Executor  — spawn(), spawn_blocking(), block_on()
└── Reactor   — sleep(), interval(), tcp_connect_addr(), register()
```

`Runtime<RK: RuntimeKit>` is the main public type. Callers interact only with this wrapper; they never import runtime-specific types directly.

### Implementors (`src/implementors/`)

Each file provides concrete `Executor` / `Reactor` impls for one runtime:

| File | Runtime | Feature flag |
|------|---------|-------------|
| `tokio.rs` | Tokio | `tokio` (default) |
| `smol.rs` | Smol | `smol` |
| `async_io.rs` | async-io (reactor only) | `async-io` |
| `async_global_executor.rs` | async-global-executor + async-io | `async-global-executor` |
| `noop.rs` | Dummy/compile-time | `noop` |
| `hickory.rs` | Hickory DNS resolver | `hickory-dns` (tokio only) |

`RuntimeParts<E, R>` (`src/util/runtime.rs`) composes a separate `Executor` + `Reactor` into a `RuntimeKit`; this is how `AGERuntime` is built.

### Task lifecycle

`Task<I: TaskImpl>` (`src/util/task.rs`) wraps runtime-specific task handles. Dropping a `Task` **detaches** it (lets it run in the background); explicit `.cancel()` is required to abort. This is intentional — see the `Drop` impl.

### Key patterns

- **Deref forwarding**: `Executor` and `Reactor` are auto-implemented for `Deref` targets, so `Arc<Runtime>`, `&Runtime`, etc. all work without extra boilerplate.
- **Feature-gated implementations**: each runtime is behind an optional feature; `default = ["tokio"]`.
- **async-compat**: the tokio implementor wraps `TcpStream` in `async-compat` to satisfy `futures-io` trait bounds.
- **Platform-specific I/O** (`src/sys/`): `AsSysFd` is Unix-only; the Windows stub returns an error for socket registration.
- **Sealed traits**: `AsyncToSocketAddrs` (`src/traits/addr.rs`) uses the sealed-trait pattern to prevent external implementations.
