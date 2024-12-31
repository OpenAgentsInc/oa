use actix_http::error::HttpError;
use actix_web::http::header::{self, HeaderValue, TryIntoHeaderValue};
use actix_web::{error::ParseError, web, HttpMessage, HttpResponse};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use std::fmt;
use tracing::{info, error, Level};
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

#[tracing::instrument(level = "info", name = "Checking database connection")]
async fn debug_connection(pool: &PgPool) {
    info!("Starting database connection check");
    
    // Get connection info
    match sqlx::query("SELECT current_database(), current_schema(), version(), current_user, inet_server_addr(), inet_server_port()").fetch_one(pool).await {
        Ok(row) => {
            let db: &str = row.try_get(0).unwrap_or("unknown");
            let schema: &str = row.try_get(1).unwrap_or("unknown");
            let version: &str = row.try_get(2).unwrap_or("unknown");
            let user: &str = row.try_get(3).unwrap_or("unknown");
            let addr: Option<String> = row.try_get(4).unwrap_or(None);
            let port: Option<i32> = row.try_get(5).unwrap_or(None);
            
            info!(
                database = %db,
                schema = %schema,
                version = %version,
                user = %user,
                server_addr = ?addr,
                server_port = ?port,
                "Database connection info"
            );
        }
        Err(e) => {
            error!(error = %e, "Failed to get database info");
        }
    }

    // List all tables in current schema
    match sqlx::query(
        "SELECT table_name, table_schema FROM information_schema.tables WHERE table_schema NOT IN ('pg_catalog', 'information_schema')"
    ).fetch_all(pool).await {
        Ok(rows) => {
            let tables: Vec<(String, String)> = rows
                .iter()
                .map(|row| (
                    row.try_get(0).unwrap_or_default(),
                    row.try_get(1).unwrap_or_default()
                ))
                .collect();
            info!(tables = ?tables, "Available tables in all schemas");
        }
        Err(e) => {
            error!(error = %e, "Failed to list tables");
        }
    }

    // Check if our specific table exists and show its definition
    match sqlx::query(
        "SELECT EXISTS (
            SELECT FROM information_schema.tables 
            WHERE table_schema = current_schema()
            AND table_name = 'shared_conversations'
        )"
    ).fetch_one(pool).await {
        Ok(row) => {
            let exists: bool = row.try_get(0).unwrap_or(false);
            info!(exists = exists, "Checked shared_conversations table existence");
            
            if exists {
                // If table exists, show its definition
                match sqlx::query(
                    "SELECT column_name, data_type, is_nullable 
                     FROM information_schema.columns 
                     WHERE table_name = 'shared_conversations'"
                ).fetch_all(pool).await {
                    Ok(columns) => {
                        let column_info: Vec<(String, String, String)> = columns
                            .iter()
                            .map(|col| (
                                col.try_get(0).unwrap_or_default(),
                                col.try_get(1).unwrap_or_default(),
                                col.try_get(2).unwrap_or_default()
                            ))
                            .collect();
                        info!(columns = ?column_info, "Table structure");
                    }
                    Err(e) => {
                        error!(error = %e, "Failed to get table structure");
                    }
                }
            }
        }
        Err(e) => {
            error!(error = %e, "Failed to check table existence");
        }
    }

    info!("Database connection check complete");
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
    info!("Starting shared conversation storage");
    
    // Debug connection before attempting insert
    debug_connection(pool).await;

    let id = Uuid::new_v4();
    info!(share_id = %id, "Generated UUID for share");
    
    // Convert messages to Value, mapping any JSON error to sqlx::Error
    let messages_json = serde_json::to_value(&payload.messages)
        .map_err(|e| {
            error!(error = %e, "Failed to serialize messages to JSON");
            sqlx::Error::Protocol(e.to_string())
        })?;
    
    let metadata_json = serde_json::json!({
        "messageCount": payload.metadata.message_count,
        "timestamp": payload.metadata.timestamp,
        "originalMetadata": payload.messages.iter().filter_map(|m| m.metadata.clone()).collect::<Vec<_>>()
    });

    // Log the exact values we're trying to insert
    info!(
        id = %id,
        chat_id = %chat_id,
        sender_npub = %nostr_pubkey.0,
        recipient_npub = %payload.recipient,
        message_count = %payload.messages.len(),
        messages = %serde_json::to_string(&messages_json).unwrap_or_default(),
        metadata = %serde_json::to_string(&metadata_json).unwrap_or_default(),
        "Attempting database insert with values"
    );

    info!("Attempting database insert");
    let result = sqlx::query!(
        r#"
        INSERT INTO shared_conversations 
        (id, chat_id, sender_npub, recipient_npub, message_count, messages, metadata)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING id
        "#,
        id,
        chat_id,
        nostr_pubkey.0,
        payload.recipient,
        payload.messages.len() as i32,
        messages_json,
        metadata_json
    )
    .fetch_one(pool)
    .await;

    match result {
        Ok(row) => {
            info!(share_id = %id, "Successfully stored shared conversation");
            Ok(id)
        }
        Err(e) => {
            error!(error = %e, "Failed to insert shared conversation");
            Err(e)
        }
    }
}

pub async fn share_chat(
    chat_id: web::Path<String>,
    nostr_pubkey: web::Header<NostrPubkey>,
    payload: web::Json<ShareRequest>,
    pool: web::Data<PgPool>,
) -> HttpResponse {
    info!(
        sender = %nostr_pubkey.0,
        chat_id = %chat_id,
        message_count = %payload.messages.len(),
        "Received share request"
    );

    match store_shared_conversation(&pool, &chat_id, &nostr_pubkey, &payload).await {
        Ok(share_id) => {
            info!(share_id = %share_id, "Share successful");
            HttpResponse::Ok().json(serde_json::json!({
                "status": "success",
                "message": "Chat shared successfully",
                "share_id": share_id.to_string()
            }))
        },
        Err(e) => {
            error!(error = %e, "Share failed");
            HttpResponse::InternalServerError().json(serde_json::json!({
                "status": "error",
                "message": "Failed to store shared conversation"
            }))
        },
    }
}