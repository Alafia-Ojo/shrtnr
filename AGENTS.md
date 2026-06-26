# AGENTS.md — project1

Single Rust 2024 binary crate. Axum + r2d2/rusqlite + HTMX dark UI URL shortener.

## Setup

`cp .env.example .env` before running. `.env` is loaded automatically at startup via `dotenvy`. Creates SQLite DB at `DATABASE_PATH` on first run. `ADMIN_TOKEN` can be set in `.env`; if unset, a random token is generated and logged on each startup.

## Admin access

Visit `/dashboard?admin_token=<token>` to see all links. Without the token, users see only links created under their `creator_id` cookie.

## Commands

| Action | Command |
|---|---|
| Run | `cargo run` |
| Lint | `cargo clippy` |
| Format check | `cargo fmt --check` |
| Docker | `docker build -t shrtnr .` |

## Structure

- `src/main.rs` — entrypoint, dotenvy init, Axum router with state.
- `src/handlers.rs` — all route handlers, inline HTML with glass-morphism dark UI.
- `src/db.rs` — SQLite schema (links + click_events), CRUD + per-user `creator_id` filter.
- `src/models.rs` — request/response types (`ShortenRequest`, `DashboardQuery`, etc.).
- `src/state.rs` — `AppState` (pool, rate limiter, admin_token).
- `src/util.rs` — helpers (rate limiter, URL validation, DB async wrappers, creator_id cookie).
- `src/qr.rs` — QR code SVG/PNG generation.
- `src/templates.rs` — INDEX_HTML home page template.
- No tests, no CI, no lib.rs, no workspace.

## User isolation

Each visitor gets a random `creator_id` cookie set on the home page. Links created via the form or API are tagged with that id. The dashboard filters links by `creator_id` unless a valid `admin_token` is provided.
