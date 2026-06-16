mod db;
mod models;

use axum::{
    Json, Router,
    extract::{Form, Path, State},
    http::{HeaderMap, StatusCode},
    response::{Html, IntoResponse, Redirect},
    routing::{get, post},
};
use models::*;
use qrcode::QrCode;
use qrcode::render::svg;
use std::sync::{Arc, Mutex};
use tower_http::cors::CorsLayer;
use tracing_subscriber::EnvFilter;

const SHORT_CODE_LEN: usize = 7;

fn qr_svg(short_code: &str) -> String {
    let url = format!("{}/{short_code}", public_url());
    let code = QrCode::new(url.as_bytes()).unwrap();
    code.render::<svg::Color>()
        .dark_color(svg::Color("#0f172a"))
        .light_color(svg::Color("#e2e8f0"))
        .min_dimensions(6, 6)
        .build()
}

fn public_url() -> String {
    std::env::var("PUBLIC_URL").unwrap_or_else(|_| "http://localhost:3000".into())
}

fn generate_code() -> String {
    nanoid::nanoid!(SHORT_CODE_LEN)
}

async fn insert_with_code(
    state: &SharedState,
    code: &str,
    url: &str,
) -> std::result::Result<(), String> {
    let state = state.clone();
    let code = code.to_owned();
    let url = url.to_owned();
    tokio::task::spawn_blocking(move || {
        let conn = state.conn.lock().unwrap();
        db::insert_link(&conn, &code, &url)
    })
    .await
    .unwrap()
    .map_err(|e| e.to_string())
}

async fn get_link_db(
    state: &SharedState,
    code: &str,
) -> std::result::Result<Option<(String, i64)>, String> {
    let state = state.clone();
    let code = code.to_owned();
    tokio::task::spawn_blocking(move || {
        let conn = state.conn.lock().unwrap();
        db::get_link(&conn, &code)
    })
    .await
    .unwrap()
    .map_err(|e| e.to_string())
}

async fn get_all_links_db(
    state: &SharedState,
) -> std::result::Result<Vec<(String, String, i64, String)>, String> {
    let state = state.clone();
    tokio::task::spawn_blocking(move || {
        let conn = state.conn.lock().unwrap();
        db::get_all_links(&conn)
    })
    .await
    .unwrap()
    .map_err(|e| e.to_string())
}

fn increment_visits_db(state: &SharedState, code: &str) {
    let state = state.clone();
    let code = code.to_owned();
    tokio::task::spawn_blocking(move || {
        let conn = state.conn.lock().unwrap();
        let _ = db::increment_visits(&conn, &code);
    });
}

const INDEX_HTML: &str = r##"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>URL Shortener</title>
  <script src="https://unpkg.com/htmx.org@2"></script>
  <link rel="preconnect" href="https://fonts.googleapis.com">
  <link href="https://fonts.googleapis.com/css2?family=Inter:opsz@14..32&display=swap" rel="stylesheet">
  <style>
    * { box-sizing: border-box; margin: 0; padding: 0; }
    body {
      font-family: 'Inter', system-ui, sans-serif;
      background: linear-gradient(135deg, #0f172a 0%, #1e293b 100%);
      min-height: 100vh;
      display: flex;
      align-items: center;
      justify-content: center;
      padding: 1rem;
      color: #e2e8f0;
    }
    .container {
      background: #1e293b;
      border: 1px solid #334155;
      border-radius: 16px;
      padding: 2.5rem;
      width: 100%;
      max-width: 480px;
      box-shadow: 0 25px 50px -12px rgba(0,0,0,0.5);
    }
    .logo {
      display: flex;
      align-items: center;
      gap: 0.5rem;
      margin-bottom: 0.5rem;
    }
    .logo svg { flex-shrink: 0; }
    h1 {
      font-size: 1.5rem;
      font-weight: 600;
      letter-spacing: -0.02em;
    }
    .subtitle {
      color: #94a3b8;
      font-size: 0.875rem;
      margin-bottom: 1.5rem;
    }
    .input-group {
      display: flex;
      flex-direction: column;
      gap: 0.5rem;
    }
    label {
      font-size: 0.875rem;
      font-weight: 500;
      color: #cbd5e1;
    }
    .input-wrap {
      display: flex;
      gap: 0.5rem;
    }
    .input-wrap input {
      flex: 1;
      padding: 0.75rem 1rem;
      font-size: 0.9375rem;
      font-family: inherit;
      background: #0f172a;
      border: 1px solid #334155;
      border-radius: 10px;
      color: #e2e8f0;
      outline: none;
      transition: border-color 0.2s;
    }
    .input-wrap input:focus {
      border-color: #6366f1;
    }
    .input-wrap input::placeholder {
      color: #475569;
    }
    .input-wrap button {
      padding: 0.75rem 1.25rem;
      font-size: 0.9375rem;
      font-family: inherit;
      font-weight: 500;
      background: linear-gradient(135deg, #6366f1, #8b5cf6);
      color: #fff;
      border: none;
      border-radius: 10px;
      cursor: pointer;
      transition: opacity 0.2s, transform 0.1s;
      white-space: nowrap;
    }
    .input-wrap button:hover { opacity: 0.9; }
    .input-wrap button:active { transform: scale(0.97); }
    .card {
      margin-top: 1.5rem;
      background: #0f172a;
      border: 1px solid #334155;
      border-radius: 12px;
      padding: 1.25rem;
    }
    .card .short-url {
      font-size: 1.125rem;
      font-weight: 600;
      word-break: break-all;
    }
    .card .short-url a {
      color: #a5b4fc;
      text-decoration: none;
    }
    .card .short-url a:hover { text-decoration: underline; }
    .card .meta {
      margin-top: 0.75rem;
      font-size: 0.8125rem;
      color: #64748b;
      display: flex;
      align-items: center;
      gap: 0.75rem;
    }
    .card .meta a {
      color: #818cf8;
      text-decoration: none;
    }
    .card .meta a:hover { text-decoration: underline; }
    .card .meta .sep { color: #334155; }
    .error {
      color: #fca5a5;
    }
    .original-link {
      font-size: 0.8125rem;
      color: #64748b;
      margin-top: 0.5rem;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }
    .original-link a {
      color: #64748b;
    }
    .footer {
      margin-top: 2rem;
      text-align: center;
      font-size: 0.75rem;
      color: #475569;
    }
    .footer a { color: #6366f1; text-decoration: none; }
    .qr-wrap {
      display: flex;
      justify-content: center;
      margin: 1rem 0 0.5rem;
    }
    .qr-wrap svg {
      width: 140px;
      height: 140px;
      border-radius: 8px;
      background: #e2e8f0;
      padding: 8px;
    }
  </style>
</head>
<body>
  <div class='container'>
    <div class='logo'>
      <svg width='28' height='28' viewBox='0 0 24 24' fill='none' stroke='#818cf8' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'>
        <path d='M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71'/>
        <path d='M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71'/>
      </svg>
      <h1>Shrtnr</h1>
    </div>
    <p class='subtitle'>Paste a long URL and get a short link</p>
    <form hx-post='/shorten' hx-target='#result' hx-swap='innerHTML'>
      <div class='input-group'>
        <label for='url'>URL to shorten</label>
        <div class='input-wrap'>
          <input type='url' id='url' name='url' placeholder='https://example.com/very/long/url' required autofocus>
          <button type='submit'>Shorten</button>
        </div>
      </div>
    </form>
    <div id='result'></div>
    <div class='footer'><a href='/dashboard'>Dashboard</a> · Powered by <a href='https://www.rust-lang.org/'>Rust</a> + <a href='https://htmx.org/'>HTMX</a></div>
  </div>
</body>
</html>"##;

struct AppState {
    conn: Mutex<rusqlite::Connection>,
}

type SharedState = Arc<AppState>;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let db_path = std::env::var("DATABASE_PATH").unwrap_or_else(|_| "urlshort.db".into());
    let conn = db::init_db(&db_path).expect("failed to initialize database");
    let state = Arc::new(AppState {
        conn: Mutex::new(conn),
    });

    let app = Router::new()
        .route("/", get(index))
        .route("/shorten", post(shorten_form))
        .route("/api/shorten", post(shorten_json))
        .route("/dashboard", get(dashboard))
        .route("/stats/{short_code}", get(stats))
        .route("/qr/{short_code}", get(qr_code))
        .route("/{short_code}", get(redirect))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".into());
    let addr = format!("0.0.0.0:{port}");
    tracing::info!("listening on {addr}");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn index() -> Html<&'static str> {
    Html(INDEX_HTML)
}

async fn shorten_form(
    State(state): State<SharedState>,
    Form(req): Form<ShortenRequest>,
) -> Html<String> {
    let short_code = loop {
        let code = generate_code();
        match insert_with_code(&state, &code, &req.url).await {
            Ok(()) => break code,
            Err(e) if e.contains("UNIQUE") => continue,
            Err(e) => return Html(format!(r#"<div class="card error">Error: {e}</div>"#)),
        }
    };

    let qr = qr_svg(&short_code);
    Html(format!(
        r##"<div class="card">
            <div class="short-url"><a href="/{code}" target="_blank">{base}/{code}</a></div>
            <div class="original-link"><a href="{url}" target="_blank">{url}</a></div>
            <div class='qr-wrap'>{qr}</div>
            <div class="meta">
                <span>{visits} visit{plural}</span>
                <span class='sep'>·</span>
                <a href='#' hx-get='/stats/{code}' hx-target='closest .card' hx-swap='outerHTML'>refresh</a>
            </div>
        </div>"##,
        code = short_code,
        base = public_url(),
        url = req.url,
        visits = 0,
        plural = "s",
        qr = qr,
    ))
}

async fn shorten_json(
    State(state): State<SharedState>,
    Json(req): Json<ShortenRequest>,
) -> impl IntoResponse {
    let short_code = loop {
        let code = generate_code();
        match insert_with_code(&state, &code, &req.url).await {
            Ok(()) => break code,
            Err(e) if e.contains("UNIQUE") => continue,
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!(ErrorResponse { error: e })),
                )
                    .into_response();
            }
        }
    };

    (
        StatusCode::CREATED,
        Json(serde_json::json!(ShortenResponse {
            short_code,
            original_url: req.url,
        })),
    )
        .into_response()
}

async fn redirect(
    State(state): State<SharedState>,
    Path(short_code): Path<String>,
) -> impl IntoResponse {
    match get_link_db(&state, &short_code).await {
        Ok(Some((url, _))) => {
            increment_visits_db(&state, &short_code);
            Redirect::temporary(&url).into_response()
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!(ErrorResponse {
                error: "not found".into()
            })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!(ErrorResponse { error: e })),
        )
            .into_response(),
    }
}

async fn qr_code(
    State(state): State<SharedState>,
    Path(short_code): Path<String>,
) -> impl IntoResponse {
    match get_link_db(&state, &short_code).await {
        Ok(Some(_)) => {
            let svg_str = qr_svg(&short_code);
            (
                [(
                    axum::http::header::CONTENT_TYPE,
                    "image/svg+xml; charset=utf-8",
                )],
                svg_str,
            )
                .into_response()
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!(ErrorResponse {
                error: "not found".into()
            })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!(ErrorResponse { error: e })),
        )
            .into_response(),
    }
}

async fn dashboard(
    State(state): State<SharedState>,
) -> impl IntoResponse {
    match get_all_links_db(&state).await {
        Ok(links) => {
            let mut rows = String::new();
            for (short_code, original_url, visits, created_at) in &links {
                rows.push_str(&format!(
                    r##"<tr>
                        <td><a href="/{code}" target="_blank">{code}</a></td>
                        <td class='url-cell'><a href="{url}" target="_blank" title="{url}">{url}</a></td>
                        <td class='num'>{visits}</td>
                        <td>{created}</td>
                    </tr>"##,
                    code = short_code,
                    url = original_url,
                    visits = visits,
                    created = created_at,
                ));
            }

            let plural = if links.len() == 1 { "" } else { "s" };

            Html(format!(
                r##"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Dashboard · Shrtnr</title>
  <link rel="preconnect" href="https://fonts.googleapis.com">
  <link href="https://fonts.googleapis.com/css2?family=Inter:opsz@14..32&display=swap" rel="stylesheet">
  <style>
    * {{ box-sizing: border-box; margin: 0; padding: 0; }}
    body {{
      font-family: 'Inter', system-ui, sans-serif;
      background: linear-gradient(135deg, #0f172a 0%, #1e293b 100%);
      min-height: 100vh;
      padding: 2rem 1rem;
      color: #e2e8f0;
    }}
    .container {{
      background: #1e293b;
      border: 1px solid #334155;
      border-radius: 16px;
      padding: 2rem;
      width: 100%;
      max-width: 900px;
      margin: 0 auto;
      box-shadow: 0 25px 50px -12px rgba(0,0,0,0.5);
    }}
    .header {{
      display: flex;
      align-items: center;
      justify-content: space-between;
      margin-bottom: 1.5rem;
    }}
    .header h1 {{
      font-size: 1.5rem;
      font-weight: 600;
    }}
    .header a {{
      color: #818cf8;
      text-decoration: none;
      font-size: 0.875rem;
    }}
    .header a:hover {{ text-decoration: underline; }}
    .count {{ color: #94a3b8; font-size: 0.875rem; margin-bottom: 1rem; }}
    table {{
      width: 100%;
      border-collapse: collapse;
    }}
    th {{
      text-align: left;
      font-size: 0.75rem;
      font-weight: 600;
      text-transform: uppercase;
      letter-spacing: 0.05em;
      color: #64748b;
      padding: 0.75rem 0.5rem;
      border-bottom: 1px solid #334155;
    }}
    td {{
      padding: 0.75rem 0.5rem;
      border-bottom: 1px solid #1e293b;
      font-size: 0.875rem;
    }}
    tr:hover td {{ background: #0f172a40; }}
    td a {{ color: #a5b4fc; text-decoration: none; }}
    td a:hover {{ text-decoration: underline; }}
    .url-cell {{
      max-width: 300px;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }}
    .url-cell a {{ color: #94a3b8; }}
    .num {{ text-align: right; font-variant-numeric: tabular-nums; color: #e2e8f0; }}
    .empty {{ text-align: center; padding: 3rem 1rem; color: #64748b; }}
    .empty a {{ color: #818cf8; }}
  </style>
</head>
<body>
  <div class="container">
    <div class="header">
      <h1>Dashboard</h1>
      <a href="/">← Shorten a URL</a>
    </div>
    <div class="count">{total} link{plural}</div>
    {table}
  </div>
</body>
</html>"##,
                total = links.len(),
                plural = plural,
                table = if links.is_empty() {
                    r##"<div class='empty'>No links yet. <a href='/'>Create one →</a></div>"##.to_string()
                } else {
                    format!(
                        r##"<table>
                            <thead><tr>
                                <th>Short code</th>
                                <th>Original URL</th>
                                <th class='num'>Visits</th>
                                <th>Created</th>
                            </tr></thead>
                            <tbody>{rows}</tbody>
                        </table>"##
                    )
                },
            ))
            .into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Html(format!("<div class='error'>Error: {e}</div>")),
        )
            .into_response(),
    }
}

async fn stats(
    State(state): State<SharedState>,
    Path(short_code): Path<String>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let is_htmx = headers.contains_key("hx-request");
    match get_link_db(&state, &short_code).await {
        Ok(Some((original_url, visits))) => {
            if is_htmx {
                let qr = qr_svg(&short_code);
                Html(format!(
                    r##"<div class="card">
                        <div class="short-url"><a href="/{code}" target="_blank">{base}/{code}</a></div>
                        <div class="original-link"><a href="{url}" target="_blank">{url}</a></div>
                        <div class='qr-wrap'>{qr}</div>
                        <div class="meta">
                            <span>{visits} visit{plural}</span>
                            <span class='sep'>·</span>
                            <a href='#' hx-get='/stats/{code}' hx-target='closest .card' hx-swap='outerHTML'>refresh</a>
                        </div>
                    </div>"##,
                    code = short_code,
                    base = public_url(),
                    url = original_url,
                    visits = visits,
                    plural = if visits == 1 { "" } else { "s" },
                    qr = qr,
                ))
                .into_response()
            } else {
                Json(serde_json::json!(StatsResponse {
                    short_code,
                    original_url,
                    visits: u64::try_from(visits).unwrap_or(0),
                }))
                .into_response()
            }
        }
        Ok(None) => {
            let msg = "not found";
            if is_htmx {
                Html(format!("<div class='card error'>{msg}</div>")).into_response()
            } else {
                (
                    StatusCode::NOT_FOUND,
                    Json(serde_json::json!(ErrorResponse {
                        error: msg.to_string()
                    })),
                )
                    .into_response()
            }
        }
        Err(e) => {
            if is_htmx {
                Html(format!("<div class='card error'>{e}</div>")).into_response()
            } else {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!(ErrorResponse { error: e })),
                )
                    .into_response()
            }
        }
    }
}
