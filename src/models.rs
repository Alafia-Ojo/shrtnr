use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct ShortenRequest {
    pub url: String,
    #[serde(default)]
    pub code: Option<String>,
}

#[derive(Serialize)]
pub struct ShortenResponse {
    pub short_code: String,
    pub original_url: String,
}

#[derive(Serialize)]
pub struct StatsResponse {
    pub short_code: String,
    pub original_url: String,
    pub visits: u64,
}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
}
