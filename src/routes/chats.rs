use actix_web::{web, HttpResponse, http::header};
use serde::Deserialize;

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

pub struct NostrPubkey(String);

impl header::Header for NostrPubkey {
    fn name() -> header::HeaderName {
        header::HeaderName::from_static("x-nostr-pubkey")
    }

    fn parse<M: header::HttpMessage>(msg: &M) -> Result<Self, header::ParseError> {
        let header = msg.headers()
            .get("x-nostr-pubkey")
            .ok_or_else(|| header::ParseError::Header)?;
        
        let value = header.to_str()
            .map_err(|_| header::ParseError::Header)?
            .to_string();
            
        Ok(NostrPubkey(value))
    }
}

pub async fn share_chat(
    chat_id: web::Path<String>,
    nostr_pubkey: web::Header<NostrPubkey>,
    payload: web::Json<ShareRequest>,
) -> HttpResponse {
    println!(
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