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
    check_rate_limit, client_ip, ensure_scheme, generate_code, get_all_links_db, get_clicks_db,
    get_link_db, get_or_create_creator_id, increment_visits_db, insert_with_code, log_click_db,
    public_url, validate_custom_code, validate_url,
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
    let short_code = match req.code.as_deref().and_then(|c| {
        let t = c.trim();
        if t.is_empty() { None } else { Some(t) }
    }) {
        Some(trimmed) => {
            if let Err(e) = validate_custom_code(trimmed) {
                return Html(format!(r#"<div class="card error">{e}</div>"#));
            }
            match insert_with_code(&state, trimmed, &url, &creator_id).await {
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
            match insert_with_code(&state, &code, &url, &creator_id).await {
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
            match insert_with_code(&state, trimmed, &url, &creator_id).await {
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
            match insert_with_code(&state, &code, &url, &creator_id).await {
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
        Ok(Some((url, _))) => {
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
    let creator = if is_admin {
        None
    } else {
        Some(get_or_create_creator_id(&headers))
    };
    let mut resp = match get_all_links_db(&state, creator.clone()).await {
        Ok(links) => {
            let mut rows = String::new();
            for (short_code, original_url, visits, created_at) in &links {
                rows.push_str(&format!(
                    r##"<tr>
                        <td><a href="/{code}" target="_blank">{code}</a></td>
                        <td class='url-cell'><a href="{url}" target="_blank" title="{url}">{url}</a></td>
                        <td class='num'>{visits}</td>
                        <td><a href="/clicks/{code}">view</a></td>
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
    @media (max-width: 500px) {{ .container {{ padding: 1rem; }} }}
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
    .table-wrap {{
      overflow-x: auto;
    }}
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
    @media (max-width: 600px) {{
      .url-cell {{ max-width: 120px; }}
    }}
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
                        r##"<div class="table-wrap"><table>
                            <thead><tr>
                                <th>Short code</th>
                                <th>Original URL</th>
                                <th class='num'>Visits</th>
                                <th>Clicks</th>
                                <th>Created</th>
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
        Ok(Some((_, v))) => v,
        Ok(None) => {
            return Html(
                r##"<!DOCTYPE html><html lang="en"><head><meta charset="UTF-8"><meta name="viewport" content="width=device-width,initial-scale=1.0"><title>Not found</title><style>body{font-family:system-ui;background:#0f172a;display:flex;align-items:center;justify-content:center;min-height:100vh;color:#e2e8f0;}</style></head><body><p>Link not found. <a href="/dashboard" style="color:#818cf8">Go to dashboard</a></p></body></html>"##,
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
    @media (max-width: 500px) {{ .container {{ padding: 1rem; }} }}
    .header {{
      display: flex;
      align-items: center;
      justify-content: space-between;
      margin-bottom: 1.5rem;
    }}
    .header h1 {{ font-size: 1.5rem; font-weight: 600; }}
    .header a {{ color: #818cf8; text-decoration: none; font-size: 0.875rem; }}
    .header a:hover {{ text-decoration: underline; }}
    .count {{ color: #94a3b8; font-size: 0.875rem; margin-bottom: 1rem; }}
    .table-wrap {{ overflow-x: auto; }}
    table {{ width: 100%; border-collapse: collapse; }}
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
      font-size: 0.8125rem;
    }}
    tr:hover td {{ background: #0f172a40; }}
    .cell-small {{ max-width: 200px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }}
    .empty {{ text-align: center; padding: 3rem 1rem; color: #64748b; }}
  </style>
</head>
<body>
  <div class="container">
    <div class="header">
      <h1>Clicks · {code}</h1>
      <a href="/dashboard">← Dashboard</a>
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
        Ok(Some((original_url, visits))) => {
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
