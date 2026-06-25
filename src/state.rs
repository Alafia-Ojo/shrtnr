use r2d2_sqlite::SqliteConnectionManager;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

pub struct AppState {
    pub pool: r2d2::Pool<SqliteConnectionManager>,
    pub rate_limiter: Mutex<HashMap<String, Vec<Instant>>>,
    pub admin_token: String,
}

pub type SharedState = Arc<AppState>;
