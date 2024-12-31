use actix_web::{web, HttpResponse};
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

pub async fn share_chat(
    chat_id: web::Path<String>,
    nostr_pubkey: web::Header<String>,
    payload: web::Json<ShareRequest>,
) -> HttpResponse {
    println!(
        "Received conversation from {} with {} messages",
        nostr_pubkey.as_str(),
        payload.messages.len()
    );
    
    HttpResponse::Ok().json(serde_json::json!({
        "status": "success",
        "message": "Chat shared successfully"
    }))
}