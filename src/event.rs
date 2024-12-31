use serde::{Deserialize, Serialize};
use crate::error::Result;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Event {
    pub id: String,
    pub pubkey: String,
    pub created_at: i64,
    pub kind: i64,
    pub tags: Vec<Vec<String>>,
    pub content: String,
    pub sig: String,
}

impl Event {
    pub fn validate(&self) -> Result<()> {
        // For now, just return Ok since we're using SQLx
        Ok(())
    }
}