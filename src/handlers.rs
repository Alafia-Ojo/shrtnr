use axum::{
    Json,
    extract::{Form, Path, Query, State},
    http::{HeaderMap, StatusCode, header},
    response::{Html, IntoResponse, Redirect},
};

use crate::models::*;
use crate::qr::{qr_png_bytes, qr_svg};
use crate::state::SharedState;
use crate::templates::INDEX_HTML;
use crate::util::{
    check_rate_limit, client_ip, delete_link_db, ensure_scheme, generate_code, get_all_links_db,
    get_clicks_db, get_link_creator_db, get_link_db, get_or_create_creator_id,
    increment_visits_db, insert_with_code, log_click_db, parse_expiry, public_url,
    validate_custom_code, validate_url,
};

pub async fn health(State(state): State<SharedState>) -> impl IntoResponse {
    let db_ok = state.pool.get().is_ok();
    let status = if db_ok {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    (
        status,
        Json(serde_json::json!({ "status": if db_ok { "ok" } else { "degraded" } })),
    )
}

pub async fn index(headers: HeaderMap) -> impl IntoResponse {
    let creator_id = get_or_create_creator_id(&headers);
    let body = INDEX_HTML.replace("__CREATOR_ID__", &creator_id);
    let cookie =
        format!("creator_id={creator_id}; Path=/; HttpOnly; SameSite=Lax; Max-Age=31536000");
    let mut resp_headers = HeaderMap::new();
    resp_headers.insert(header::SET_COOKIE, cookie.parse().unwrap());
    (resp_headers, Html(body))
}

pub async fn shorten_form(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Form(req): Form<ShortenRequest>,
) -> Html<String> {
    let ip = client_ip(&headers);
    if let Err(e) = check_rate_limit(&state, &ip) {
        return Html(format!(r#"<div class="card error">{e}</div>"#));
    }

    let url = ensure_scheme(&req.url);

    if let Err(e) = validate_url(&url).await {
        return Html(format!(r#"<div class="card error">{e}</div>"#));
    }
    let creator_id = match req.creator_id.as_deref().filter(|s| !s.is_empty()) {
        Some(id) => id.to_owned(),
        None => get_or_create_creator_id(&headers),
    };
    let expires_hours = req.expiry.as_deref().and_then(parse_expiry);
    let short_code = match req.code.as_deref().and_then(|c| {
        let t = c.trim();
        if t.is_empty() { None } else { Some(t) }
    }) {
        Some(trimmed) => {
            if let Err(e) = validate_custom_code(trimmed) {
                return Html(format!(r#"<div class="card error">{e}</div>"#));
            }
            match insert_with_code(&state, trimmed, &url, &creator_id, expires_hours).await {
                Ok(()) => trimmed.to_owned(),
                Err(e) if e.contains("UNIQUE") => {
                    return Html(
                        r#"<div class="card error">custom code already taken</div>"#.into(),
                    );
                }
                Err(e) => return Html(format!(r#"<div class="card error">Error: {e}</div>"#)),
            }
        }
        None => loop {
            let code = generate_code();
            match insert_with_code(&state, &code, &url, &creator_id, expires_hours).await {
                Ok(()) => break code,
                Err(e) if e.contains("UNIQUE") => continue,
                Err(e) => return Html(format!(r#"<div class="card error">Error: {e}</div>"#)),
            }
        },
    };

    let qr = match qr_svg(&short_code) {
        Ok(svg) => svg,
        Err(e) => return Html(format!(r#"<div class="card error">{e}</div>"#)),
    };
    Html(format!(
        r##"<div class="card">
            <div class="short-url"><a href="/{code}" target="_blank">{base}/{code}</a> <button class='copy-btn' onclick="copy('{base}/{code}', this)">copy</button></div>
            <div class="original-link"><a href="{url}" target="_blank">{url}</a></div>
            <div class='qr-wrap'>{qr}</div>
            <div class='qr-download'><a href='/qr/{code}/png' download='{code}.png'>Download QR</a></div>
            <div class="meta">
                <span>{visits} visit{plural}</span>
                <span class='sep'>·</span>
                <a href='#' hx-get='/stats/{code}' hx-target='closest .card' hx-swap='outerHTML'>refresh</a>
                <span class='sep'>·</span>
                <a href='/clicks/{code}'>clicks</a>
            </div>
        </div>"##,
        code = short_code,
        base = public_url(),
        url = url,
        visits = 0,
        plural = "s",
        qr = qr,
    ))
}

pub async fn shorten_json(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Json(req): Json<ShortenRequest>,
) -> impl IntoResponse {
    let ip = client_ip(&headers);
    if let Err(e) = check_rate_limit(&state, &ip) {
        return (
            StatusCode::TOO_MANY_REQUESTS,
            Json(serde_json::json!(ErrorResponse { error: e })),
        )
            .into_response();
    }

    let url = ensure_scheme(&req.url);

    if let Err(e) = validate_url(&url).await {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!(ErrorResponse { error: e })),
        )
            .into_response();
    }

    let creator_id = match req.creator_id.as_deref().filter(|s| !s.is_empty()) {
        Some(id) => id.to_owned(),
        None => get_or_create_creator_id(&headers),
    };
    let expires_hours = req.expiry.as_deref().and_then(parse_expiry);
    let short_code = match req.code.as_deref().and_then(|c| {
        let t = c.trim();
        if t.is_empty() { None } else { Some(t) }
    }) {
        Some(trimmed) => {
            if let Err(e) = validate_custom_code(trimmed) {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!(ErrorResponse { error: e })),
                )
                    .into_response();
            }
            match insert_with_code(&state, trimmed, &url, &creator_id, expires_hours).await {
                Ok(()) => trimmed.to_owned(),
                Err(e) if e.contains("UNIQUE") => {
                    return (
                        StatusCode::CONFLICT,
                        Json(serde_json::json!(ErrorResponse {
                            error: "custom code already taken".into()
                        })),
                    )
                        .into_response();
                }
                Err(e) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(serde_json::json!(ErrorResponse { error: e })),
                    )
                        .into_response();
                }
            }
        }
        None => loop {
            let code = generate_code();
            match insert_with_code(&state, &code, &url, &creator_id, expires_hours).await {
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
        },
    };

    (
        StatusCode::CREATED,
        Json(serde_json::json!(ShortenResponse {
            short_code,
            original_url: url,
        })),
    )
        .into_response()
}

pub async fn redirect(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Path(short_code): Path<String>,
) -> impl IntoResponse {
    match get_link_db(&state, &short_code).await {
        Ok(Some((url, _, _, is_expired))) => {
            if is_expired {
                return (
                    StatusCode::GONE,
                    Html(r##"<!DOCTYPE html><html lang="en"><head><meta charset="UTF-8"><meta name="viewport" content="width=device-width,initial-scale=1.0"><title>Link expired</title><style>body{font-family:system-ui;background:#0b1120;display:flex;align-items:center;justify-content:center;min-height:100vh;color:#e2e8f0;text-align:center;padding:1rem;}</style></head><body><div><h1 style="font-size:1.5rem;margin-bottom:0.5rem;">This link has expired</h1><p style="color:#64748b;">The link you followed is no longer available.</p><p style="margin-top:1rem;"><a href="/" style="color:#818cf8;">Create a new short link</a></p></div></body></html>"##),
                )
                    .into_response();
            }
            let referrer = headers
                .get("referer")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("");
            let user_agent = headers
                .get("user-agent")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("");
            let ip = client_ip(&headers);
            log_click_db(&state, &short_code, referrer, user_agent, &ip);
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

pub async fn qr_code(
    State(state): State<SharedState>,
    Path(short_code): Path<String>,
) -> impl IntoResponse {
    match get_link_db(&state, &short_code).await {
        Ok(Some(_)) => match qr_svg(&short_code) {
            Ok(svg_str) => (
                [(
                    axum::http::header::CONTENT_TYPE,
                    "image/svg+xml; charset=utf-8",
                )],
                svg_str,
            )
                .into_response(),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!(ErrorResponse { error: e })),
            )
                .into_response(),
        },
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

pub async fn qr_code_png(
    State(state): State<SharedState>,
    Path(short_code): Path<String>,
) -> impl IntoResponse {
    match get_link_db(&state, &short_code).await {
        Ok(Some(_)) => match qr_png_bytes(&short_code) {
            Ok(png) => {
                let disposition = format!("attachment; filename=\"{short_code}.png\"");
                let headers = [
                    (axum::http::header::CONTENT_TYPE, "image/png"),
                    (
                        axum::http::header::CONTENT_DISPOSITION,
                        disposition.as_str(),
                    ),
                ];
                (headers, png).into_response()
            }
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!(ErrorResponse { error: e })),
            )
                .into_response(),
        },
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

pub async fn dashboard(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Query(query): Query<DashboardQuery>,
) -> impl IntoResponse {
    let is_admin = query
        .admin_token
        .as_deref()
        .map(|t| t == state.admin_token)
        .unwrap_or(false);
    let admin_token_str = query.admin_token.as_deref().unwrap_or_default().to_owned();
    let creator = if is_admin {
        None
    } else {
        Some(get_or_create_creator_id(&headers))
    };
    let mut resp = match get_all_links_db(&state, creator.clone()).await {
        Ok(links) => {
            let total_visits: i64 = links.iter().map(|(_, _, v, _, _, _)| v).sum();
            let mut rows = String::new();
            let base = public_url();
            for (short_code, original_url, visits, created_at, expires_at, is_expired) in &links {
                let expiry = match (expires_at, is_expired) {
                    (Some(date), false) => format!("exp. {}", date),
                    (Some(_), true) => "expired".to_owned(),
                    (None, _) => "never".to_owned(),
                };
                let qr = qr_svg(short_code).unwrap_or_default();
                let del_url = if is_admin {
                    format!("/delete/{short_code}?admin_token={admin_token_str}")
                } else {
                    format!("/delete/{short_code}")
                };
                rows.push_str(&format!(
                    r##"<tr>
                        <td class="code-cell"><a href="/{code}" target="_blank">{code}</a></td>
                        <td class="url-cell"><a href="{url}" target="_blank" title="{url}">{url}</a></td>
                        <td class="num">{visits}</td>
                        <td class="action-cell">
                          <button class="tbl-copy" onclick="copy('{base}/{code}',this)" title="Copy short URL">⎘</button>
                          <a href="/clicks/{code}" class="tbl-link">clicks</a>
                        </td>
                        <td class="date-cell">{created}</td>
                        <td class="qr-cell"><a href="/qr/{code}/png" target="_blank" title="Download QR PNG">{qr}</a></td>
                        <td class="expiry-cell{expired_cls}">{expiry}</td>
                        <td class="del-cell"><button class="delete-btn" hx-delete="{del_url}" hx-target="closest tr" hx-swap="delete" hx-confirm="Delete this link?">✕</button></td>
                    </tr>"##,
                    code = short_code,
                    base = base,
                    url = original_url,
                    visits = visits,
                    created = created_at,
                    expiry = expiry,
                    qr = qr,
                    del_url = del_url,
                    expired_cls = if *is_expired { " expired" } else { "" },
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
  <script src="https://unpkg.com/htmx.org@2"></script>
  <link rel="preconnect" href="https://fonts.googleapis.com">
  <link href="https://fonts.googleapis.com/css2?family=Inter:opsz@14..32&display=swap" rel="stylesheet">
  <style>
    * {{ box-sizing: border-box; margin: 0; padding: 0; }}
    body {{
      font-family: 'Inter', system-ui, sans-serif;
      background: #0b1120;
      min-height: 100vh;
      padding: 2rem 1rem;
      color: #e2e8f0;
    }}
    body::before {{
      content: ''; position: fixed; inset: 0; z-index: -1;
      background: radial-gradient(800px circle at 50% 0%, rgba(99,102,241,0.06) 0%, transparent 70%),
                  radial-gradient(500px circle at 80% 80%, rgba(139,92,246,0.04) 0%, transparent 60%);
    }}
    .container {{
      background: rgba(30,41,59,0.7);
      backdrop-filter: blur(16px);
      -webkit-backdrop-filter: blur(16px);
      border: 1px solid rgba(51,65,85,0.5);
      border-radius: 20px;
      padding: 2rem;
      width: 100%;
      max-width: 960px;
      margin: 0 auto;
      box-shadow: 0 25px 50px -12px rgba(0,0,0,0.5), inset 0 1px 0 rgba(255,255,255,0.03);
    }}
    @media (max-width: 600px) {{ .container {{ padding: 1rem; border-radius: 16px; }} }}
    .header {{
      display: flex;
      align-items: center;
      justify-content: space-between;
      margin-bottom: 1.5rem;
    }}
    .header h1 {{
      font-size: 1.5rem;
      font-weight: 700;
      letter-spacing: -0.03em;
      background: linear-gradient(135deg, #e2e8f0 0%, #94a3b8 100%);
      -webkit-background-clip: text;
      -webkit-text-fill-color: transparent;
      background-clip: text;
    }}
    .header a {{
      color: #818cf8;
      text-decoration: none;
      font-size: 0.875rem;
      transition: color 0.2s;
    }}
    .header a:hover {{ color: #a5b4fc; text-decoration: underline; }}
    .stats {{
      display: flex;
      gap: 1rem;
      margin-bottom: 1.5rem;
    }}
    @media (max-width: 500px) {{ .stats {{ flex-direction: column; gap: 0.5rem; }} }}
    .stat-card {{
      flex: 1;
      background: rgba(15,23,42,0.6);
      border: 1px solid rgba(51,65,85,0.4);
      border-radius: 12px;
      padding: 1rem 1.25rem;
      display: flex;
      flex-direction: column;
      gap: 0.25rem;
    }}
    .stat-value {{
      font-size: 1.5rem;
      font-weight: 700;
      letter-spacing: -0.02em;
      color: #e2e8f0;
    }}
    .stat-label {{
      font-size: 0.75rem;
      font-weight: 500;
      text-transform: uppercase;
      letter-spacing: 0.05em;
      color: #64748b;
    }}
    table {{
      width: 100%;
      border-collapse: collapse;
    }}
    th {{
      text-align: left;
      font-size: 0.6875rem;
      font-weight: 600;
      text-transform: uppercase;
      letter-spacing: 0.06em;
      color: #64748b;
      padding: 0.75rem 0.5rem 0.5rem;
      border-bottom: 1px solid rgba(51,65,85,0.5);
    }}
    td {{
      padding: 0.75rem 0.5rem;
      border-bottom: 1px solid rgba(30,41,59,0.5);
      font-size: 0.875rem;
      vertical-align: middle;
    }}
    tr:last-child td {{ border-bottom: none; }}
    tr:hover td {{ background: rgba(15,23,42,0.4); }}
    td a {{ color: #a5b4fc; text-decoration: none; }}
    td a:hover {{ text-decoration: underline; }}
    .table-wrap {{ overflow-x: auto; margin: 0 -0.5rem; padding: 0 0.5rem; }}
    .code-cell {{ white-space: nowrap; font-weight: 500; }}
    .url-cell {{
      max-width: 280px;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }}
    .url-cell a {{ color: #94a3b8; font-size: 0.8125rem; }}
    .date-cell {{ white-space: nowrap; color: #64748b; font-size: 0.8125rem; }}
    .action-cell {{ white-space: nowrap; }}
    .num {{ text-align: right; font-variant-numeric: tabular-nums; color: #e2e8f0; font-weight: 500; }}
    .empty {{ text-align: center; padding: 3rem 1rem; color: #64748b; }}
    .empty a {{ color: #818cf8; }}
    .expiry-cell {{ white-space: nowrap; color: #64748b; font-size: 0.75rem; }}
    .expiry-cell.expired {{ color: #f87171; font-weight: 500; }}
    .qr-cell {{ padding: 0.25rem 0.5rem !important; text-align: center; }}
    .qr-cell a {{ display: inline-block; }}
    .qr-cell svg {{ display: block; width: 36px; height: 36px; border-radius: 4px; background: #e2e8f0; padding: 3px; transition: transform 0.15s; }}
    .qr-cell a:hover svg {{ transform: scale(1.15); }}
    .tbl-copy {{
      background: none; border: 1px solid rgba(51,65,85,0.4); color: #64748b;
      font-size: 0.875rem; padding: 0.125rem 0.375rem; border-radius: 6px;
      cursor: pointer; transition: all 0.15s; vertical-align: middle; line-height: 1;
    }}
    .tbl-copy:hover {{ border-color: rgba(99,102,241,0.3); color: #a5b4fc; background: rgba(99,102,241,0.08); }}
    .tbl-link {{
      font-size: 0.8125rem; margin-left: 0.5rem; color: #64748b !important;
    }}
    .tbl-link:hover {{ color: #818cf8 !important; }}
    .delete-btn {{
      background: none; border: 1px solid rgba(239,68,68,0.3); color: #f87171;
      font-size: 0.75rem; padding: 0.125rem 0.375rem; border-radius: 6px;
      cursor: pointer; transition: all 0.15s; vertical-align: middle; line-height: 1;
      margin-left: 0.25rem;
    }}
    .delete-btn:hover {{ border-color: rgba(239,68,68,0.6); color: #fca5a5; background: rgba(239,68,68,0.1); }}
    .del-cell {{ text-align: center; padding-left: 0.25rem; padding-right: 0.25rem; }}
    .toast-container {{
      position: fixed; top: 1rem; right: 1rem; z-index: 999;
      display: flex; flex-direction: column; gap: 0.5rem;
      pointer-events: none;
    }}
    .toast {{
      pointer-events: auto;
      background: rgba(30,41,59,0.95);
      backdrop-filter: blur(12px);
      border: 1px solid rgba(51,65,85,0.5);
      border-radius: 10px;
      padding: 0.75rem 1rem;
      font-size: 0.875rem;
      color: #e2e8f0;
      box-shadow: 0 10px 30px -10px rgba(0,0,0,0.5);
      animation: toast-in 0.3s ease, toast-out 0.3s ease 3.7s forwards;
      max-width: 360px;
    }}
    .toast.error {{ border-color: rgba(239,68,68,0.5); }}
    @keyframes toast-in {{ from {{ opacity: 0; transform: translateX(100%); }} to {{ opacity: 1; transform: translateX(0); }} }}
    @keyframes toast-out {{ from {{ opacity: 1; }} to {{ opacity: 0; transform: translateX(100%); }} }}
    .loading-bar {{
      position: fixed; top: 0; left: 0; z-index: 9999;
      width: 0; height: 3px;
      background: linear-gradient(90deg, #6366f1, #8b5cf6, #a78bfa);
      border-radius: 0 2px 2px 0;
      transition: width 0.3s ease, opacity 0.3s;
      opacity: 0;
      box-shadow: 0 0 12px rgba(99,102,241,0.5);
    }}
    .loading-bar.active {{ width: 60%; opacity: 1; }}
    .loading-bar.done {{ width: 100%; opacity: 0; transition: width 0.15s, opacity 0.4s 0.15s; }}
    @media (max-width: 640px) {{
      .url-cell {{ max-width: 100px; }}
      .date-cell {{ display: none; }}
      th:nth-child(5), td:nth-child(5) {{ display: none; }}
      .qr-cell, th:nth-child(6), td:nth-child(6) {{ display: none; }}
    }}
  </style>
</head>
<body>
  <div class="loading-bar" id="loading-bar"></div>
  <div class="toast-container" id="toast-container"></div>
  <div class="container">
    <div class="header">
      <h1>Dashboard</h1>
      <a href="/">← Shorten a URL</a>
    </div>
    <div class="stats">
      <div class="stat-card">
        <span class="stat-value">{total}</span>
        <span class="stat-label">Link{plural}</span>
      </div>
      <div class="stat-card">
        <span class="stat-value">{total_visits}</span>
        <span class="stat-label">Total Visits</span>
      </div>
    </div>
    {table}
  </div>
  <script>
    function copy(url, btn) {{
      navigator.clipboard.writeText(url);
      btn.textContent = '✓';
      btn.style.borderColor = 'rgba(52,211,153,0.4)';
      btn.style.color = '#34d399';
      setTimeout(function() {{
        btn.textContent = '⎘';
        btn.style.borderColor = '';
        btn.style.color = '';
      }}, 1500);
    }}
    function showToast(msg, type) {{
      var c = document.getElementById('toast-container');
      if (!c) return;
      var t = document.createElement('div');
      t.className = 'toast' + (type ? ' ' + type : '');
      t.textContent = msg;
      c.appendChild(t);
      setTimeout(function() {{ if (t.parentNode) t.parentNode.removeChild(t); }}, 4200);
    }}
    var loadBar = document.getElementById('loading-bar');
    document.body.addEventListener('htmx:beforeRequest', function() {{
      loadBar.className = 'loading-bar active';
    }});
    document.body.addEventListener('htmx:afterRequest', function() {{
      loadBar.className = 'loading-bar done';
      setTimeout(function() {{ loadBar.className = 'loading-bar'; }}, 600);
    }});
    document.body.addEventListener('htmx:beforeSwap', function(evt) {{
      if (evt.detail.xhr && evt.detail.xhr.status >= 400) {{
        showToast(evt.detail.serverResponse || 'Request failed', 'error');
        evt.detail.shouldSwap = false;
      }}
    }});
  </script>
</body>
</html>"##,
                total = links.len(),
                plural = plural,
                total_visits = total_visits,
                table = if links.is_empty() {
                    r##"<div class='empty'>No links yet. <a href='/'>Create one →</a></div>"##.to_string()
                } else {
                    format!(
                        r##"<div class="table-wrap"><table>
                            <thead><tr>
                                <th>Short code</th>
                                <th>Original URL</th>
                                <th class='num'>Visits</th>
                                <th>Actions</th>
                                <th>Created</th>
                                <th>QR</th>
                                <th>Expires</th>
                                <th></th>
                            </tr></thead>
                            <tbody>{rows}</tbody>
                        </table></div>"##
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
    };
    if let Some(id) = creator {
        let cookie = format!("creator_id={id}; Path=/; HttpOnly; SameSite=Lax; Max-Age=31536000");
        resp.headers_mut()
            .insert(header::SET_COOKIE, cookie.parse().unwrap());
    }
    resp
}

pub async fn clicks(
    State(state): State<SharedState>,
    Path(short_code): Path<String>,
) -> impl IntoResponse {
    let visits = match get_link_db(&state, &short_code).await {
        Ok(Some((_, v, _, _))) => v,
        Ok(None) => {
            return Html(
                r##"<!DOCTYPE html><html lang="en"><head><meta charset="UTF-8"><meta name="viewport" content="width=device-width,initial-scale=1.0"><title>Not found</title><style>body{font-family:system-ui;background:#0b1120;display:flex;align-items:center;justify-content:center;min-height:100vh;color:#e2e8f0;}</style></head><body><p>Link not found. <a href="/dashboard" style="color:#818cf8">Go to dashboard</a></p></body></html>"##,
            )
            .into_response();
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Html(format!("<div class='card error'>Error: {e}</div>")),
            )
                .into_response();
        }
    };

    let clicks_list = get_clicks_db(&state, &short_code).await.unwrap_or_default();

    let mut rows = String::new();
    for (referrer, user_agent, ip, clicked_at) in &clicks_list {
        rows.push_str(&format!(
            r##"<tr>
                        <td class='cell-small' title="{ua}">{ua}</td>
                        <td class='cell-small'>{ref}</td>
                        <td class='cell-small'>{ip}</td>
                        <td class='cell-small'>{at}</td>
                    </tr>"##,
            ua = user_agent,
            ref = referrer,
            ip = ip,
            at = clicked_at,
        ));
    }

    Html(format!(
                r##"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Clicks · {code} · Shrtnr</title>
  <link rel="preconnect" href="https://fonts.googleapis.com">
  <link href="https://fonts.googleapis.com/css2?family=Inter:opsz@14..32&display=swap" rel="stylesheet">
  <style>
    * {{ box-sizing: border-box; margin: 0; padding: 0; }}
    body {{
      font-family: 'Inter', system-ui, sans-serif;
      background: #0b1120;
      min-height: 100vh;
      padding: 2rem 1rem;
      color: #e2e8f0;
    }}
    body::before {{
      content: ''; position: fixed; inset: 0; z-index: -1;
      background: radial-gradient(600px circle at 50% 0%, rgba(99,102,241,0.06) 0%, transparent 70%);
    }}
    .container {{
      background: rgba(30,41,59,0.7);
      backdrop-filter: blur(16px);
      -webkit-backdrop-filter: blur(16px);
      border: 1px solid rgba(51,65,85,0.5);
      border-radius: 20px;
      padding: 2rem;
      width: 100%;
      max-width: 960px;
      margin: 0 auto;
      box-shadow: 0 25px 50px -12px rgba(0,0,0,0.5), inset 0 1px 0 rgba(255,255,255,0.03);
    }}
    @media (max-width: 600px) {{ .container {{ padding: 1rem; border-radius: 16px; }} }}
    .header {{
      display: flex;
      align-items: center;
      justify-content: space-between;
      margin-bottom: 1.5rem;
      flex-wrap: wrap;
      gap: 0.5rem;
    }}
    .header h1 {{
      font-size: 1.25rem;
      font-weight: 700;
      letter-spacing: -0.02em;
    }}
    .header a {{ color: #818cf8; text-decoration: none; font-size: 0.875rem; }}
    .header a:hover {{ text-decoration: underline; }}
    .count {{ color: #64748b; font-size: 0.875rem; margin-bottom: 1rem; }}
    .table-wrap {{ overflow-x: auto; }}
    table {{ width: 100%; border-collapse: collapse; }}
    th {{
      text-align: left;
      font-size: 0.6875rem;
      font-weight: 600;
      text-transform: uppercase;
      letter-spacing: 0.06em;
      color: #64748b;
      padding: 0.75rem 0.5rem 0.5rem;
      border-bottom: 1px solid rgba(51,65,85,0.5);
    }}
    td {{
      padding: 0.75rem 0.5rem;
      border-bottom: 1px solid rgba(30,41,59,0.5);
      font-size: 0.8125rem;
      vertical-align: middle;
    }}
    tr:last-child td {{ border-bottom: none; }}
    tr:hover td {{ background: rgba(15,23,42,0.4); }}
    .cell-small {{ max-width: 220px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }}
    .empty {{ text-align: center; padding: 3rem 1rem; color: #64748b; }}
    @media (max-width: 500px) {{
      td:nth-child(2), th:nth-child(2) {{ display: none; }}
      td:nth-child(4), th:nth-child(4) {{ display: none; }}
    }}
  </style>
</head>
<body>
  <div class="container">
    <div class="header">
      <h1>Clicks &middot; {code}</h1>
      <a href="/dashboard">&larr; Dashboard</a>
    </div>
    <div class="count">{visits} click{plural}</div>
    {table}
  </div>
</body>
</html>"##,
                code = short_code,
                visits = visits,
                plural = if visits == 1 { "" } else { "s" },
                table = if clicks_list.is_empty() {
                    r##"<div class='empty'>No clicks yet. Share your link!</div>"##.to_string()
                } else {
                    format!(
                        r##"<div class="table-wrap"><table>
                            <thead><tr>
                                <th>User agent</th>
                                <th>Referrer</th>
                                <th>IP</th>
                                <th>Time</th>
                            </tr></thead>
                            <tbody>{rows}</tbody>
                        </table></div>"##
                    )
                },
            ))
            .into_response()
}

pub async fn stats(
    State(state): State<SharedState>,
    Path(short_code): Path<String>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let is_htmx = headers.contains_key("hx-request");
    match get_link_db(&state, &short_code).await {
        Ok(Some((original_url, visits, _, _))) => {
            if is_htmx {
                match qr_svg(&short_code) {
                    Ok(qr) => Html(format!(
                        r##"<div class="card">
                            <div class="short-url"><a href="/{code}" target="_blank">{base}/{code}</a> <button class='copy-btn' onclick="copy('{base}/{code}', this)">copy</button></div>
                            <div class="original-link"><a href="{url}" target="_blank">{url}</a></div>
                            <div class='qr-wrap'>{qr}</div>
                            <div class='qr-download'><a href='/qr/{code}/png' download='{code}.png'>Download QR</a></div>
                            <div class="meta">
                                <span>{visits} visit{plural}</span>
                                <span class='sep'>·</span>
                                <a href='#' hx-get='/stats/{code}' hx-target='closest .card' hx-swap='outerHTML'>refresh</a>
                                <span class='sep'>·</span>
                                <a href='/clicks/{code}'>clicks</a>
                            </div>
                        </div>"##,
                        code = short_code,
                        base = public_url(),
                        url = original_url,
                        visits = visits,
                        plural = if visits == 1 { "" } else { "s" },
                        qr = qr,
                    ))
                    .into_response(),
                    Err(e) => Html(format!("<div class='card error'>{e}</div>")).into_response(),
                }
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

pub async fn delete_link(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Path(short_code): Path<String>,
    Query(query): Query<DashboardQuery>,
) -> impl IntoResponse {
    let is_admin = query
        .admin_token
        .as_deref()
        .map(|t| t == state.admin_token)
        .unwrap_or(false);
    if !is_admin {
        let creator_id = get_or_create_creator_id(&headers);
        match get_link_creator_db(&state, &short_code).await {
            Ok(Some(c)) if c == creator_id => {}
            _ => {
                return (StatusCode::FORBIDDEN, "forbidden").into_response();
            }
        }
    }
    match delete_link_db(&state, &short_code).await {
        Ok(()) => StatusCode::OK.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
    }
}
