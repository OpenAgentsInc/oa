
//! Event persistence and querying
use crate::config::Settings;
use crate::error::{Error, Result};
use crate::event::Event;
use crate::repo::postgres::{PostgresPool, PostgresRepo};
use crate::repo::NostrRepo;
use crate::server::NostrMetrics;
use log::LevelFilter;
use sqlx::pool::PoolOptions;
use sqlx::postgres::PgConnectOptions;
use sqlx::ConnectOptions;
use std::sync::Arc;
use std::time::Duration;
use tracing::info;

pub mod builder;
pub mod types;
pub mod writer;

pub use builder::build_repo;
pub use types::*;
pub use writer::{db_writer, SubmittedEvent};

/// Serialized event associated with a specific subscription request.
#[derive(PartialEq, Eq, Debug, Clone)]
pub struct QueryResult {
    /// Subscription identifier
    pub sub_id: String,
    /// Serialized event
    pub event: String,
}

async fn build_postgres_pool(settings: &Settings, metrics: NostrMetrics) -> PostgresRepo {
    let options: PgConnectOptions = settings.database.connection.as_str().parse().unwrap();
    let mut options = options.log_statements(LevelFilter::Debug);
    options = options.log_slow_statements(LevelFilter::Warn, Duration::from_secs(60));

    let pool: PostgresPool = PoolOptions::new()
        .max_connections(settings.database.max_conn)
        .min_connections(settings.database.min_conn)
        .idle_timeout(Duration::from_secs(60))
        .connect_with(options)
        .await
        .unwrap();

    let write_pool: PostgresPool = match &settings.database.connection_write {
        Some(cfg_write) => {
            let options_write: PgConnectOptions = cfg_write.as_str().parse().unwrap();
            let mut options_write = options_write.log_statements(LevelFilter::Debug);
            options_write = options_write.log_slow_statements(LevelFilter::Warn, Duration::from_secs(60));

            PoolOptions::new()
                .max_connections(settings.database.max_conn)
                .min_connections(settings.database.min_conn)
                .idle_timeout(Duration::from_secs(60))
                .connect_with(options_write)
                .await
                .unwrap()
        }
        None => pool.clone(),
    };

    let repo = PostgresRepo::new(pool, write_pool, metrics);

    // Panic on migration failure
    let version = repo.migrate_up().await.unwrap();
    info!("Postgres migration completed, at v{}", version);
    // startup scheduled tasks
    repo.start().await.ok();
    repo
}
