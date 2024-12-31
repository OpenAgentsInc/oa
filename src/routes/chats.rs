use actix_http::error::HttpError;
use actix_web::http::header::{self, HeaderValue, TryIntoHeaderValue};
use actix_web::{error::ParseError, web, HttpMessage, HttpResponse};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::fmt;
use uuid::Uuid;

#[derive(Debug, Deserialize, Serialize)]
#[allow(dead_code)]
pub struct ShareRequest {
    recipient: String,
    messages: Vec<ChatMessage>,
    metadata: ShareMetadata,
}

#[derive(Debug, Deserialize, Serialize)]
#[allow(dead_code)]
pub struct ChatMessage {
    id: String,
    role: String,
    content: String,
    #[serde(rename = "createdAt")]
    created_at: i64,
    metadata: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize, Serialize)]
#[allow(dead_code)]
pub struct ShareMetadata {
    #[serde(rename = "messageCount")]
    message_count: usize,
    timestamp: i64,
}

#[derive(Clone, Debug)]
pub struct NostrPubkey(String);

impl fmt::Display for NostrPubkey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryIntoHeaderValue for NostrPubkey {
    type Error = HttpError;

    fn try_into_value(self) -> Result<HeaderValue, Self::Error> {
        HeaderValue::from_str(&self.0).map_err(|e| e.into())
    }
}

impl header::Header for NostrPubkey {
    fn name() -> header::HeaderName {
        header::HeaderName::from_static("x-nostr-pubkey")
    }

    fn parse<M: HttpMessage>(msg: &M) -> Result<Self, ParseError> {
        let header = msg
            .headers()
            .get("x-nostr-pubkey")
            .ok_or(ParseError::Header)?;

        let value = header.to_str().map_err(|_| ParseError::Header)?.to_string();

        Ok(NostrPubkey(value))
    }
}

#[tracing::instrument(
    name = "Storing shared conversation",
    skip(pool, payload),
    fields(
        sender_npub = %nostr_pubkey.0,
        recipient_npub = %payload.recipient,
        chat_id = %chat_id,
        message_count = %payload.messages.len()
    )
)]
async fn store_shared_conversation(
    pool: &PgPool,
    chat_id: &str,
    nostr_pubkey: &NostrPubkey,
    payload: &ShareRequest,
) -> Result<Uuid, sqlx::Error> {
    let id = Uuid::new_v4();
    
    // Convert messages to Value, mapping any JSON error to sqlx::Error
    let messages_json = serde_json::to_value(&payload.messages)
        .map_err(|e| sqlx::Error::Protocol(e.to_string()))?;
    
    let metadata_json = serde_json::json!({
        "messageCount": payload.metadata.message_count,
        "timestamp": payload.metadata.timestamp,
        "originalMetadata": payload.messages.iter().filter_map(|m| m.metadata.clone()).collect::<Vec<_>>()
    });

    sqlx::query!(
        r#"
        INSERT INTO shared_conversations 
        (id, chat_id, sender_npub, recipient_npub, message_count, messages, metadata)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#,
        id,
        chat_id,
        nostr_pubkey.0,
        payload.recipient,
        payload.messages.len() as i32,
        messages_json,
        metadata_json
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to store shared conversation: {:?}", e);
        e
    })?;

    Ok(id)
}

pub async fn share_chat(
    chat_id: web::Path<String>,
    nostr_pubkey: web::Header<NostrPubkey>,
    payload: web::Json<ShareRequest>,
    pool: web::Data<PgPool>,
) -> HttpResponse {
    tracing::info!(
        "Received conversation from {} with {} messages for chat_id {}",
        nostr_pubkey.0,
        payload.messages.len(),
        chat_id
    );

    match store_shared_conversation(&pool, &chat_id, &nostr_pubkey, &payload).await {
        Ok(share_id) => HttpResponse::Ok().json(serde_json::json!({
            "status": "success",
            "message": "Chat shared successfully",
            "share_id": share_id.to_string()
        })),
        Err(_) => HttpResponse::InternalServerError().json(serde_json::json!({
            "status": "error",
            "message": "Failed to store shared conversation"
        })),
    }
}