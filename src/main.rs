mod db;
mod handlers;
mod models;
mod qr;
mod state;
mod templates;
mod util;

use axum::Router;
use axum::routing::{delete, get, post};
use r2d2_sqlite::SqliteConnectionManager;
use state::AppState;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tracing_subscriber::EnvFilter;

use handlers::{
    clicks, dashboard, delete_link, health, index, qr_code, qr_code_png, redirect, shorten_form,
    shorten_json, stats,
};

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let db_path = std::env::var("DATABASE_PATH").unwrap_or_else(|_| "urlshort.db".into());
    let manager = SqliteConnectionManager::file(db_path);
    let pool = r2d2::Pool::builder()
        .max_size(8)
        .build(manager)
        .expect("failed to create database pool");
    db::init_schema(&pool.get().expect("failed to get initial connection"))
        .expect("failed to initialize database schema");

    let admin_token = std::env::var("ADMIN_TOKEN").unwrap_or_else(|_| {
        let token = nanoid::nanoid!(32);
        tracing::warn!("ADMIN_TOKEN not set — generated random token for this session");
        tracing::warn!("admin: /dashboard?admin_token={token}");
        tracing::warn!("set ADMIN_TOKEN in .env to make it permanent");
        token
    });

    let state = Arc::new(AppState {
        pool,
        rate_limiter: std::sync::Mutex::new(std::collections::HashMap::new()),
        admin_token,
    });

    let app = Router::new()
        .route("/", get(index))
        .route("/health", get(health))
        .route("/shorten", post(shorten_form))
        .route("/api/shorten", post(shorten_json))
        .route("/dashboard", get(dashboard))
        .route("/stats/{short_code}", get(stats))
        .route("/clicks/{short_code}", get(clicks))
        .route("/qr/{short_code}", get(qr_code))
        .route("/qr/{short_code}/png", get(qr_code_png))
        .route("/delete/{short_code}", delete(delete_link))
        .route("/{short_code}", get(redirect))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".into());
    let addr = format!("0.0.0.0:{port}");
    tracing::info!("listening on {addr}");
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("failed to bind to address {addr}");
    axum::serve(listener, app).await.expect("server failed");
}
