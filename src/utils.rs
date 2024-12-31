//! Utility functions
use std::time::{SystemTime, UNIX_EPOCH};

/// Get current unix timestamp
#[must_use]
pub fn unix_time() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs()
}

/// Extract host from URL string
#[must_use]
pub fn host_str(url: &str) -> Option<&str> {
    url.split("://").nth(1)?.split('/').next()
}