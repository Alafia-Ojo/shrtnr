use crate::state::SharedState;
use axum::http::HeaderMap;
use std::time::{Duration, Instant};

pub fn get_or_create_creator_id(headers: &HeaderMap) -> String {
    if let Some(cookie) = headers.get("cookie").and_then(|v| v.to_str().ok()) {
        for pair in cookie.split(';') {
            let mut parts = pair.splitn(2, '=');
            let name = parts.next().unwrap_or("").trim();
            let value = parts.next().unwrap_or("").trim();
            if name == "creator_id" && !value.is_empty() {
                return value.to_owned();
            }
        }
    }
    nanoid::nanoid!(21)
}

const SHORT_CODE_LEN: usize = 7;
const RATE_LIMIT_REQUESTS: usize = 10;
const RATE_LIMIT_WINDOW: Duration = Duration::from_secs(60);

pub fn public_url() -> String {
    std::env::var("PUBLIC_URL").unwrap_or_else(|_| "http://localhost:3000".into())
}

pub fn ensure_scheme(url: &str) -> String {
    if url.starts_with("http://") || url.starts_with("https://") {
        url.to_owned()
    } else {
        format!("https://{url}")
    }
}

pub fn validate_custom_code(code: &str) -> std::result::Result<(), String> {
    if code.len() < 3 {
        return Err("custom code must be at least 3 characters".into());
    }
    if code.len() > 20 {
        return Err("custom code must be 20 characters or less".into());
    }
    if !code
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        return Err(
            "custom code can only contain letters, numbers, hyphens, and underscores".into(),
        );
    }
    Ok(())
}

pub fn generate_code() -> String {
    nanoid::nanoid!(SHORT_CODE_LEN)
}

pub fn parse_expiry_minutes(value: &str) -> Option<i64> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    trimmed.parse::<i64>().ok().filter(|m| *m > 0)
}

pub fn client_ip(headers: &HeaderMap) -> String {
    headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(',').next())
        .map(|s| s.trim().to_owned())
        .unwrap_or_else(|| "unknown".into())
}

pub fn check_rate_limit(state: &SharedState, ip: &str) -> std::result::Result<(), String> {
    let now = Instant::now();
    let mut limiter = state
        .rate_limiter
        .lock()
        .map_err(|_| "internal error: rate limiter unavailable".to_string())?;
    let timestamps = limiter.entry(ip.to_owned()).or_default();

    timestamps.retain(|t| now.duration_since(*t) < RATE_LIMIT_WINDOW);

    if timestamps.len() >= RATE_LIMIT_REQUESTS {
        return Err("rate limit exceeded. try again later.".into());
    }

    timestamps.push(now);
    Ok(())
}

pub async fn validate_url(url: &str) -> std::result::Result<(), String> {
    let url = url.to_owned();
    tokio::task::spawn_blocking(move || {
        let ua = "Mozilla/5.0 (compatible; Shrtnr/1.0)";
        match attohttpc::head(&url)
            .header("User-Agent", ua)
            .timeout(std::time::Duration::from_secs(5))
            .send()
        {
            Ok(r) if r.is_success() || r.status().is_redirection() => return Ok(()),
            _ => {}
        }
        attohttpc::get(&url)
            .header("User-Agent", ua)
            .header("Range", "bytes=0-0")
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .map(|_| ())
            .map_err(|_| "unable to reach URL. check that it exists.".to_string())
    })
    .await
    .map_err(|e| format!("internal error: {e}"))?
}

pub async fn insert_with_code(
    state: &SharedState,
    code: &str,
    url: &str,
    creator_id: &str,
    expires_minutes: Option<i64>,
) -> std::result::Result<(), String> {
    let pool = state.pool.clone();
    let code = code.to_owned();
    let url = url.to_owned();
    let creator_id = creator_id.to_owned();
    tokio::task::spawn_blocking(move || {
        let conn = pool.get().map_err(|e| e.to_string())?;
        crate::db::insert_link(&conn, &code, &url, &creator_id, expires_minutes)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("internal error: {e}"))?
}

pub async fn get_link_db(
    state: &SharedState,
    code: &str,
) -> std::result::Result<Option<crate::db::LinkInfo>, String> {
    let pool = state.pool.clone();
    let code = code.to_owned();
    tokio::task::spawn_blocking(move || {
        let conn = pool.get().map_err(|e| e.to_string())?;
        crate::db::get_link(&conn, &code).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("internal error: {e}"))?
}

pub async fn get_all_links_db(
    state: &SharedState,
    creator_id: Option<String>,
) -> std::result::Result<Vec<crate::db::LinkRow>, String> {
    let pool = state.pool.clone();
    tokio::task::spawn_blocking(move || {
        let conn = pool.get().map_err(|e| e.to_string())?;
        crate::db::get_all_links(&conn, creator_id.as_deref()).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("internal error: {e}"))?
}

pub fn log_click_db(
    state: &SharedState,
    short_code: &str,
    referrer: &str,
    user_agent: &str,
    ip: &str,
) {
    let pool = state.pool.clone();
    let short_code = short_code.to_owned();
    let referrer = referrer.to_owned();
    let user_agent = user_agent.to_owned();
    let ip = ip.to_owned();
    tokio::task::spawn_blocking(move || {
        if let Ok(conn) = pool.get() {
            let _ = crate::db::log_click(&conn, &short_code, &referrer, &user_agent, &ip);
        }
    });
}

pub async fn get_clicks_db(
    state: &SharedState,
    short_code: &str,
) -> std::result::Result<Vec<(String, String, String, String)>, String> {
    let pool = state.pool.clone();
    let short_code = short_code.to_owned();
    tokio::task::spawn_blocking(move || {
        let conn = pool.get().map_err(|e| e.to_string())?;
        crate::db::get_clicks(&conn, &short_code).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("internal error: {e}"))?
}

pub fn increment_visits_db(state: &SharedState, code: &str) {
    let pool = state.pool.clone();
    let code = code.to_owned();
    tokio::task::spawn_blocking(move || {
        if let Ok(conn) = pool.get() {
            let _ = crate::db::increment_visits(&conn, &code);
        }
    });
}

pub async fn delete_link_db(
    state: &SharedState,
    short_code: &str,
) -> std::result::Result<(), String> {
    let pool = state.pool.clone();
    let short_code = short_code.to_owned();
    tokio::task::spawn_blocking(move || {
        let conn = pool.get().map_err(|e| e.to_string())?;
        crate::db::delete_link(&conn, &short_code).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("internal error: {e}"))?
}

pub async fn get_link_creator_db(
    state: &SharedState,
    short_code: &str,
) -> std::result::Result<Option<String>, String> {
    let pool = state.pool.clone();
    let short_code = short_code.to_owned();
    tokio::task::spawn_blocking(move || {
        let conn = pool.get().map_err(|e| e.to_string())?;
        crate::db::get_link_creator(&conn, &short_code).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("internal error: {e}"))?
}
