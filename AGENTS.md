# AGENTS.md — project1

Single Rust binary crate (edition 2024). No dependencies, no tests, no CI.

## Commands

| Action | Command |
|---|---|
| Build | `cargo build` |
| Run | `cargo run` |
| Check | `cargo check` |
| Test | `cargo test` |
| Lint | `cargo clippy` |
| Format | `cargo fmt` |
| Format check | `cargo fmt --check` |

## Conventions

- **Commit `Cargo.lock`** — this is a binary crate, not a library.
- Add `rust-toolchain.toml` to pin the toolchain if reproducibility matters.
- Tests go in `src/` (unit) or `tests/` (integration) following standard Cargo conventions.

## Structure

- `src/main.rs` — single binary entrypoint.
- No library (`lib.rs`), no workspaces, no subcrates.
