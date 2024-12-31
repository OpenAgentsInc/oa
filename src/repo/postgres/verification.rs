use crate::error::{Error, Result};
use crate::nip05::{Nip05Name, VerificationRecord};
use crate::repo::now_jitter;
use chrono::{DateTime, Utc};
use sqlx::postgres::PgRow;
use sqlx::{Error as SqlxError, FromRow, Row};
use tracing::info;

impl super::PostgresRepo {
    pub(crate) async fn create_verification_record(
        &self,
        event_id: &str,
        name: &str,
    ) -> Result<()> {
        let mut tx = self.conn_write.begin().await?;

        sqlx::query("DELETE FROM user_verification WHERE \"name\" = $1")
            .bind(name)
            .execute(&mut *tx)
            .await?;

        sqlx::query("INSERT INTO user_verification (event_id, \"name\", verified_at) VALUES ($1, $2, now())")
            .bind(hex::decode(event_id).ok())
            .bind(name)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        info!("saved new verification record for ({:?})", name);
        Ok(())
    }

    pub(crate) async fn update_verification_timestamp(&self, id: u64) -> Result<()> {
        // add some jitter to the verification to prevent everything from stacking up together.
        let verify_time = now_jitter(600);

        // update verification time and reset any failure count
        sqlx::query("UPDATE user_verification SET verified_at = $1, fail_count = 0 WHERE id = $2")
            .bind(Utc.timestamp_opt(verify_time as i64, 0).unwrap())
            .bind(id as i64)
            .execute(&self.conn_write)
            .await?;

        info!("verification updated for {}", id);
        Ok(())
    }

    pub(crate) async fn fail_verification(&self, id: u64) -> Result<()> {
        sqlx::query("UPDATE user_verification SET failed_at = now(), fail_count = fail_count + 1 WHERE id = $1")
            .bind(id as i64)
            .execute(&self.conn_write)
            .await?;
        Ok(())
    }

    pub(crate) async fn delete_verification(&self, id: u64) -> Result<()> {
        sqlx::query("DELETE FROM user_verification WHERE id = $1")
            .bind(id as i64)
            .execute(&self.conn_write)
            .await?;
        Ok(())
    }

    pub(crate) async fn get_latest_user_verification(
        &self,
        pub_key: &str,
    ) -> Result<VerificationRecord> {
        let query = r#"SELECT
            v.id,
            v."name",
            e.id as event_id,
            e.pub_key,
            e.created_at,
            v.verified_at,
            v.failed_at,
            v.fail_count
            FROM user_verification v
            LEFT JOIN "event" e ON e.id = v.event_id
            WHERE e.pub_key = $1
            ORDER BY e.created_at DESC, v.verified_at DESC, v.failed_at DESC
            LIMIT 1"#;
        sqlx::query_as::<_, VerificationRecord>(query)
            .bind(hex::decode(pub_key).ok())
            .fetch_optional(&self.conn)
            .await?
            .ok_or(Error::SqlxError(SqlxError::RowNotFound))
    }

    pub(crate) async fn get_oldest_user_verification(
        &self,
        before: u64,
    ) -> Result<VerificationRecord> {
        let query = r#"SELECT
            v.id,
            v."name",
            e.id as event_id,
            e.pub_key,
            e.created_at,
            v.verified_at,
            v.failed_at,
            v.fail_count
            FROM user_verification v
            LEFT JOIN "event" e ON e.id = v.event_id
                WHERE (v.verified_at < $1 OR v.verified_at IS NULL)
                AND (v.failed_at < $1 OR v.failed_at IS NULL)
            ORDER BY v.verified_at ASC, v.failed_at ASC
            LIMIT 1"#;
        sqlx::query_as::<_, VerificationRecord>(query)
            .bind(Utc.timestamp_opt(before as i64, 0).unwrap())
            .fetch_optional(&self.conn)
            .await?
            .ok_or(Error::SqlxError(SqlxError::RowNotFound))
    }
}

impl FromRow<'_, PgRow> for VerificationRecord {
    fn from_row(row: &'_ PgRow) -> std::result::Result<Self, SqlxError> {
        let name = Nip05Name::try_from(row.get::<'_, &str, &str>("name"))
            .or(Err(SqlxError::RowNotFound))?;
        Ok(VerificationRecord {
            rowid: row.get::<'_, i64, &str>("id") as u64,
            name,
            address: hex::encode(row.get::<'_, Vec<u8>, &str>("pub_key")),
            event: hex::encode(row.get::<'_, Vec<u8>, &str>("event_id")),
            event_created: row.get::<'_, DateTime<Utc>, &str>("created_at").timestamp() as u64,
            last_success: match row.try_get::<'_, DateTime<Utc>, &str>("verified_at") {
                Ok(x) => Some(x.timestamp() as u64),
                _ => None,
            },
            last_failure: match row.try_get::<'_, DateTime<Utc>, &str>("failed_at") {
                Ok(x) => Some(x.timestamp() as u64),
                _ => None,
            },
            failure_count: row.get::<'_, i32, &str>("fail_count") as u64,
        })
    }
}
