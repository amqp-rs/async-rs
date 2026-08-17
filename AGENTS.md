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

#### One tokio kit, one tokio runtime

`Tokio` can be handed both an owned `Runtime` and a `Handle`, and only the owned one can be driven by `block_on`, so the owned one wins whichever order the two were given in. `Tokio::bound_handle()` is the single place that decision is made: **every** entry point — `block_on`, `spawn`, `spawn_blocking`, `sleep`, `interval`, `register`, `tcp_connect_addr` — must resolve through it (or through `handle()`/`enter()`/`require_enter()`, which are built on it) rather than reading `self.handle` directly, or the kit ends up straddling two runtimes. Guarded by `one_kit_binds_everything_to_the_same_runtime`.

An *unbound* kit falls back to the ambient runtime, and when that fallback happens matters: `sleep` and `interval` capture their handle as they are constructed, so they resolve eagerly, while `tcp_connect_addr` only touches the reactor once polled, so it resolves inside `InTokioContext::poll`. Resolving connect eagerly condemns a future built on a plain thread even when a real runtime later polls it.

Whichever way a runtime turns out to be missing, the message is `NO_RUNTIME`: the entry points returning an `io::Result` report it, the rest panic with it, and none of them let tokio's own "there is no reactor running" out.

### Task lifecycle

`Task<I: TaskImpl>` (`src/util/task.rs`) wraps runtime-specific task handles. Dropping a `Task` **detaches** it (lets it run in the background); explicit `.cancel()` is required to abort. This is intentional — see the `Drop` impl.

Awaiting a `Task` yields its output, which leaves no room to report a failure, so on tokio, smol and async-global-executor a failed task **panics in the awaiting task**: one which panicked resumes its panic, and one which was canceled, or whose runtime went away, panics too. `Noop` is the exception — it runs nothing, so its tasks simply never complete. `.cancel()` takes `&mut self`, so awaiting afterwards is expressible and panics rather than failing to compile; the same goes for a `.cancel()` which was itself dropped before it completed, since it gives up the underlying handle on its first poll.

`.cancel()` itself is the other exception: it returns `Option<T>`, so on all three backends a task which panicked comes back as `None`, indistinguishable from one which was simply cancelled in time. That is a gap in the trait, not a backend bug — `async-task`'s own `FallibleTask` does the same — and closing it needs the output type to have room for a failure (see `FIXME.md`).

### Key patterns

- **Deref forwarding**: `Executor` and `Reactor` are auto-implemented for `Deref` targets, so `Arc<Runtime>`, `&Runtime`, etc. all work without extra boilerplate.
- **Feature-gated implementations**: each runtime is behind an optional feature; `default = ["tokio"]`.
- **async-compat**: the tokio implementor wraps `TcpStream` in `async-compat` to satisfy `futures-io` trait bounds.
- **Platform-specific I/O** (`src/sys/`): `AsSysFd` is Unix-only; the Windows stub returns an error for socket registration.
- **Sealed traits**: `AsyncToSocketAddrs` (`src/traits/addr.rs`) uses the sealed-trait pattern to prevent external implementations.
