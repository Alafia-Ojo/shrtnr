# 🔗 Shrtnr

> Because your URLs are too long and your patience is too short.

Shrtnr is a URL shortener written in Rust. Paste a monster URL, get a tiny one. It comes with QR codes, visit tracking, and a dashboard — all wrapped in a dark theme that's easy on the eyes.

## ✨ Features

- **Shorten URLs** — turn `https://example.com/some/absurdly/long/path?with=params&that=hurt` into `localhost:3000/xY3kLm9`
- **QR codes** — every short link gets a scannable QR code, because you're living in the future
- **Visit tracking** — know how many people clicked your link (and judge your content accordingly)
- **Dashboard** — see all your links in one place, like a control room for tiny URLs
- **REST API** — `POST /api/shorten` if you're the programmatic type
- **Dark mode** — because your terminal is already dark and your eyes deserve consistency

## 🚀 Quick Start

```bash
cargo run
```

Then open `http://localhost:3000` and shorten away.

## 🔧 Configuration

| Variable | Default | What it does |
|---|---|---|
| `PORT` | `3000` | What port to listen on |
| `DATABASE_PATH` | `urlshort.db` | Where to store the SQLite database |
| `PUBLIC_URL` | `http://localhost:3000` | The public-facing URL (used in QR codes and short links) |

## 📡 API

### `POST /api/shorten`

```json
{ "url": "https://example.com/very/long/url" }
```

Returns `201` with:

```json
{
  "short_code": "xY3kLm9",
  "original_url": "https://example.com/very/long/url"
}
```

### `GET /stats/{short_code}`

Returns visit stats as JSON.

### `GET /qr/{short_code}`

Returns an SVG QR code.

## 🐳 Deploy

A `Dockerfile` is included for easy deployment on Railway, Fly.io, or wherever you like to put your containers.

## 🛠️ Tech Stack

| Thing | What |
|---|---|
| Language | Rust (edition 2024) |
| Web framework | Axum |
| Database | SQLite via rusqlite |
| QR codes | qrcode crate (SVG output) |
| Frontend | HTMX + vanilla CSS |
| IDs | nanoid |

## 📜 License

Do whatever you want with it. It's code. It wants to be free.
