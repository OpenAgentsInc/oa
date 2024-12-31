use crate::error::{Error, Result};
use nostr::key::Keys;
use sqlx::Error::RowNotFound;

impl super::PostgresRepo {
    pub(crate) async fn create_account(&self, pub_key: &Keys) -> Result<bool> {
        let pub_key = pub_key.public_key().to_string();
        let mut tx = self.conn_write.begin().await?;

        let result = sqlx::query("INSERT INTO account (pubkey, balance) VALUES ($1, 0);")
            .bind(pub_key)
            .execute(&mut *tx)
            .await;

        let success = match result {
            Ok(res) => {
                tx.commit().await?;
                res.rows_affected() == 1
            }
            Err(_err) => false,
        };

        Ok(success)
    }

    pub(crate) async fn admit_account(&self, pub_key: &Keys, admission_cost: u64) -> Result<()> {
        let pub_key = pub_key.public_key().to_string();
        sqlx::query(
            "UPDATE account SET is_admitted = TRUE, balance = balance - $1 WHERE pubkey = $2",
        )
        .bind(admission_cost as i64)
        .bind(pub_key)
        .execute(&self.conn_write)
        .await?;
        Ok(())
    }

    pub(crate) async fn get_account_balance(&self, pub_key: &Keys) -> Result<(bool, u64)> {
        let pub_key = pub_key.public_key().to_string();
        let query = r#"SELECT
            is_admitted,
            balance
            FROM account
            WHERE pubkey = $1
            LIMIT 1"#;

        let result = sqlx::query_as::<_, (bool, i64)>(query)
            .bind(pub_key)
            .fetch_optional(&self.conn_write)
            .await?
            .ok_or(Error::SqlxError(RowNotFound))?;

        Ok((result.0, result.1 as u64))
    }

    pub(crate) async fn update_account_balance(
        &self,
        pub_key: &Keys,
        positive: bool,
        new_balance: u64,
    ) -> Result<()> {
        let pub_key = pub_key.public_key().to_string();
        match positive {
            true => {
                sqlx::query("UPDATE account SET balance = balance + $1 WHERE pubkey = $2")
                    .bind(new_balance as i64)
                    .bind(pub_key)
                    .execute(&self.conn_write)
                    .await?
            }
            false => {
                sqlx::query("UPDATE account SET balance = balance - $1 WHERE pubkey = $2")
                    .bind(new_balance as i64)
                    .bind(pub_key)
                    .execute(&self.conn_write)
                    .await?
            }
        };
        Ok(())
    }
}