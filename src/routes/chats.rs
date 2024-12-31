use actix_web::{web, HttpResponse, HttpMessage, error::ParseError};
use serde::Deserialize;
use actix_web::http::header::{self, HeaderValue};
use std::fmt;

#[derive(Deserialize)]
pub struct ShareRequest {
    recipient: String,
    messages: Vec<ChatMessage>,
    metadata: ShareMetadata,
}

#[derive(Deserialize)]
pub struct ChatMessage {
    id: String,
    role: String,
    content: String,
    #[serde(rename = "createdAt")]
    created_at: i64,
    metadata: Option<serde_json::Value>,
}

#[derive(Deserialize)]
pub struct ShareMetadata {
    #[serde(rename = "messageCount")]
    message_count: usize,
    timestamp: i64,
}

#[derive(Clone)]
pub struct NostrPubkey(String);

impl fmt::Display for NostrPubkey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl header::Header for NostrPubkey {
    fn name() -> header::HeaderName {
        header::HeaderName::from_static("x-nostr-pubkey")
    }

    fn parse<M: HttpMessage>(msg: &M) -> Result<Self, ParseError> {
        let header = msg.headers()
            .get("x-nostr-pubkey")
            .ok_or(ParseError::Header)?;
        
        let value = header.to_str()
            .map_err(|_| ParseError::Header)?
            .to_string();
            
        Ok(NostrPubkey(value))
    }
}

pub async fn share_chat(
    chat_id: web::Path<String>,
    nostr_pubkey: web::Header<NostrPubkey>,
    payload: web::Json<ShareRequest>,
) -> HttpResponse {
    tracing::info!(
        "Received conversation from {} with {} messages for chat_id {}",
        nostr_pubkey.0,
        payload.messages.len(),
        chat_id
    );
    
    HttpResponse::Ok().json(serde_json::json!({
        "status": "success",
        "message": "Chat shared successfully"
    }))
}