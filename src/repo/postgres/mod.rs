 pub mod account;
pub mod event;
pub mod invoice;
pub mod query;
pub mod verification;

use crate::error::Result;
use crate::repo::postgres_migration::run_migrations;
use crate::server::NostrMetrics;
use async_trait::async_trait;
use std::time::{Duration, Instant};
use tracing::{info, warn};

pub type PostgresPool = sqlx::pool::Pool<sqlx::Postgres>;

pub struct PostgresRepo {
    pub(crate) conn: PostgresPool,
    pub(crate) conn_write: PostgresPool,
    pub(crate) metrics: NostrMetrics,
}

impl PostgresRepo {
    pub fn new(c: PostgresPool, cw: PostgresPool, m: NostrMetrics) -> PostgresRepo {
        PostgresRepo {
            conn: c,
            conn_write: cw,
            metrics: m,
        }
    }
}

/// Cleanup expired events on a regular basis
async fn cleanup_expired(conn: PostgresPool, frequency: Duration) -> Result<()> {
    tokio::task::spawn(async move {
        loop {
            tokio::select! {
                _ = tokio::time::sleep(frequency) => {
                    let start = Instant::now();
                    let exp_res = delete_expired(conn.clone()).await;
                    match exp_res {
                        Ok(exp_count) => {
                            if exp_count > 0 {
                                info!("removed {} expired events in: {:?}", exp_count, start.elapsed());
                            }
                        },
                        Err(e) => {
                            warn!("could not remove expired events due to error: {:?}", e);
                        }
                    }
                }
            };
        }
    });
    Ok(())
}

/// One-time deletion of all expired events
async fn delete_expired(conn: PostgresPool) -> Result<u64> {
    let mut tx = conn.begin().await?;
    let update_count = sqlx::query("DELETE FROM \"event\" WHERE expires_at <= $1;")
        .bind(
            chrono::Utc::now(),
        )
        .execute(&mut *tx)
        .await?
        .rows_affected();
    tx.commit().await?;
    Ok(update_count)
}

#[async_trait]
impl crate::repo::NostrRepo for PostgresRepo {
    async fn start(&self) -> Result<()> {
        // begin a cleanup task for expired events.
        cleanup_expired(self.conn_write.clone(), Duration::from_secs(600)).await?;
        Ok(())
    }

    async fn migrate_up(&self) -> Result<usize> {
        Ok(run_migrations(&self.conn_write).await?)
    }

    async fn optimize_db(&self) -> Result<()> {
        // Not implemented for Postgres
        Ok(())
    }

    async fn write_event(&self, _event: crate::event::Event) -> Result<()> {
        unimplemented!()
    }

    async fn query_subscription(&self, _filter: crate::filter::Filter, _limit: Option<u64>, _offset: Option<u64>, _since: Option<i64>, _until: Option<i64>) -> Result<Vec<crate::event::Event>> {
        unimplemented!()
    }

    async fn create_verification_record(&self, _pubkey: String, _code: String) -> Result<()> {
        unimplemented!()
    }

    async fn update_verification_timestamp(&self, _pubkey: String) -> Result<()> {
        unimplemented!()
    }

    async fn fail_verification(&self, _pubkey: String) -> Result<()> {
        unimplemented!()
    }

    async fn delete_verification(&self, _pubkey: String) -> Result<()> {
        unimplemented!()
    }

    async fn get_latest_user_verification(&self, _pubkey: String) -> Result<Option<crate::verification::Verification>> {
        unimplemented!()
    }

    async fn get_oldest_user_verification(&self, _pubkey: String) -> Result<Option<crate::verification::Verification>> {
        unimplemented!()
    }

    async fn create_account(&self, _pubkey: String) -> Result<()> {
        unimplemented!()
    }

    async fn admit_account(&self, _pubkey: String, _amount: u64) -> Result<()> {
        unimplemented!()
    }

    async fn get_account_balance(&self, _pubkey: String) -> Result<u64> {
        unimplemented!()
    }

    async fn update_account_balance(&self, _pubkey: String, _amount: i64, _reason: String) -> Result<()> {
        unimplemented!()
    }

    async fn create_invoice_record(&self, _invoice: crate::invoice::Invoice, _pubkey: String) -> Result<()> {
        unimplemented!()
    }

    async fn update_invoice(&self, _invoice: crate::invoice::Invoice, _pubkey: String) -> Result<()> {
        unimplemented!()
    }

    async fn get_unpaid_invoice(&self, _pubkey: String) -> Result<Option<crate::invoice::Invoice>> {
        unimplemented!()
    }
}
