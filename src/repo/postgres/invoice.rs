use crate::error::{Error, Result};
use crate::payment::{InvoiceInfo, InvoiceStatus};
use nostr::key::Keys;
use sqlx::Error::RowNotFound;
use tracing::debug;

impl super::PostgresRepo {
    pub(crate) async fn create_invoice_record(&self, pub_key: &Keys, invoice_info: InvoiceInfo) -> Result<()> {
        let pub_key = pub_key.public_key().to_string();
        let mut tx = self.conn_write.begin().await?;

        sqlx::query(
            "INSERT INTO invoice (pubkey, payment_hash, amount, status, description, created_at, invoice) VALUES ($1, $2, $3, $4, $5, now(), $6)",
        )
            .bind(pub_key)
            .bind(invoice_info.payment_hash)
            .bind(invoice_info.amount as i64)
            .bind(invoice_info.status)
            .bind(invoice_info.memo)
            .bind(invoice_info.bolt11)
            .execute(&mut *tx)
            .await?;

        debug!("Invoice added");

        tx.commit().await?;
        Ok(())
    }

    pub(crate) async fn update_invoice(&self, payment_hash: &str, status: InvoiceStatus) -> Result<String> {
        debug!("Payment Hash: {}", payment_hash);
        let query = "SELECT pubkey, status, amount FROM invoice WHERE payment_hash=$1;";
        let (pubkey, prev_invoice_status, amount) =
            sqlx::query_as::<_, (String, InvoiceStatus, i64)>(query)
                .bind(payment_hash)
                .fetch_optional(&self.conn_write)
                .await?
                .ok_or(Error::SqlxError(RowNotFound))?;

        // If the invoice is paid update the confirmed at timestamp
        let query = if status.eq(&InvoiceStatus::Paid) {
            "UPDATE invoice SET status=$1, confirmed_at = now() WHERE payment_hash=$2;"
        } else {
            "UPDATE invoice SET status=$1 WHERE payment_hash=$2;"
        };

        sqlx::query(query)
            .bind(&status)
            .bind(payment_hash)
            .execute(&self.conn_write)
            .await?;

        if prev_invoice_status.eq(&InvoiceStatus::Unpaid) && status.eq(&InvoiceStatus::Paid) {
            sqlx::query("UPDATE account SET balance = balance + $1 WHERE pubkey = $2")
                .bind(amount)
                .bind(&pubkey)
                .execute(&self.conn_write)
                .await?;
        }

        Ok(pubkey)
    }

    pub(crate) async fn get_unpaid_invoice(&self, pubkey: &Keys) -> Result<Option<InvoiceInfo>> {
        let query = r#"
SELECT amount, payment_hash, description, invoice
FROM invoice
WHERE pubkey = $1
ORDER BY created_at DESC
LIMIT 1;
        "#;
        match sqlx::query_as::<_, (i64, String, String, String)>(query)
            .bind(pubkey.public_key().to_string())
            .fetch_optional(&self.conn_write)
            .await?
        {
            Some((amount, payment_hash, description, invoice)) => Ok(Some(InvoiceInfo {
                pubkey: pubkey.public_key().to_string(),
                payment_hash,
                bolt11: invoice,
                amount: amount as u64,
                status: InvoiceStatus::Unpaid,
                memo: description,
                confirmed_at: None,
            })),
            None => Ok(None),
        }
    }
}