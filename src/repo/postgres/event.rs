
use crate::error::Result;
use crate::event::{single_char_tagname, Event};
use crate::utils::{is_hex, is_lower_hex};
use chrono::{TimeZone, Utc};
use sqlx::QueryBuilder;
use std::time::Instant;
use tracing::info;

impl super::PostgresRepo {
    pub(crate) async fn write_event(&self, e: &Event) -> Result<u64> {
        // start transaction
        let mut tx = self.conn_write.begin().await?;
        let start = Instant::now();

        // get relevant fields from event and convert to blobs.
        let id_blob = hex::decode(&e.id).ok();
        let pubkey_blob: Option<Vec<u8>> = hex::decode(&e.pubkey).ok();
        let delegator_blob: Option<Vec<u8>> =
            e.delegated_by.as_ref().and_then(|d| hex::decode(d).ok());
        let event_str = serde_json::to_string(&e).unwrap();

        // determine if this event would be shadowed by an existing
        // replaceable event or parameterized replaceable event.
        if e.is_replaceable() {
            let repl_count = sqlx::query(
                "SELECT e.id FROM event e WHERE e.pub_key=$1 AND e.kind=$2 AND e.created_at >= $3 LIMIT 1;")
                .bind(&pubkey_blob)
                .bind(e.kind as i64)
                .bind(Utc.timestamp(e.created_at as i64, 0))
                .fetch_optional(&mut *tx)
                .await?;
            if repl_count.is_some() {
                return Ok(0);
            }
        }
        if let Some(d_tag) = e.distinct_param() {
            let repl_count: i64 = if is_lower_hex(&d_tag) && (d_tag.len() % 2 == 0) {
                sqlx::query_scalar(
                    "SELECT count(*) AS count FROM event e LEFT JOIN tag t ON e.id=t.event_id WHERE e.pub_key=$1 AND e.kind=$2 AND t.name='d' AND t.value_hex=$3 AND e.created_at >= $4 LIMIT 1;")
                    .bind(hex::decode(&e.pubkey).ok())
                    .bind(e.kind as i64)
                    .bind(hex::decode(d_tag).ok())
                    .bind(Utc.timestamp(e.created_at as i64, 0))
                    .fetch_one(&mut *tx)
                    .await?
            } else {
                sqlx::query_scalar(
                    "SELECT count(*) AS count FROM event e LEFT JOIN tag t ON e.id=t.event_id WHERE e.pub_key=$1 AND e.kind=$2 AND t.name='d' AND t.value=$3 AND e.created_at >= $4 LIMIT 1;")
                    .bind(hex::decode(&e.pubkey).ok())
                    .bind(e.kind as i64)
                    .bind(d_tag.as_bytes())
                    .bind(Utc.timestamp(e.created_at as i64, 0))
                    .fetch_one(&mut *tx)
                    .await?
            };
            // if any rows were returned, then some newer event with
            // the same author/kind/tag value exist, and we can ignore
            // this event.
            if repl_count > 0 {
                return Ok(0);
            }
        }
        // ignore if the event hash is a duplicate.
        let mut ins_count = sqlx::query(
            r#"INSERT INTO "event"
(id, pub_key, created_at, expires_at, kind, "content", delegated_by)
VALUES($1, $2, $3, $4, $5, $6, $7)
ON CONFLICT (id) DO NOTHING"#,
        )
        .bind(&id_blob)
        .bind(&pubkey_blob)
        .bind(Utc.timestamp(e.created_at as i64, 0))
        .bind(
            e.expiration()
                .and_then(|x| Utc.timestamp_opt(x as i64, 0).single()),
        )
        .bind(e.kind as i64)
        .bind(event_str.into_bytes())
        .bind(delegator_blob)
        .execute(&mut *tx)
        .await?
        .rows_affected();

        if ins_count == 0 {
            // if the event was a duplicate, no need to insert event or
            // pubkey references.  This will abort the txn.
            return Ok(0);
        }

        // add all tags to the tag table
        for tag in e.tags.iter() {
            // ensure we have 2 values.
            if tag.len() >= 2 {
                let tag_name = &tag[0];
                let tag_val = &tag[1];
                // only single-char tags are searchable
                let tag_char_opt = single_char_tagname(tag_name);
                match &tag_char_opt {
                    Some(_) => {
                        // if tag value is lowercase hex;
                        if is_lower_hex(tag_val) && (tag_val.len() % 2 == 0) {
                            sqlx::query("INSERT INTO tag (event_id, \"name\", value, value_hex) VALUES($1, $2, NULL, $3) \
                    ON CONFLICT (event_id, \"name\", value, value_hex) DO NOTHING")
                                .bind(&id_blob)
                                .bind(tag_name)
                                .bind(hex::decode(tag_val).ok())
                                .execute(&mut *tx)
                                .await?;
                        } else {
                            sqlx::query("INSERT INTO tag (event_id, \"name\", value, value_hex) VALUES($1, $2, $3, NULL) \
                    ON CONFLICT (event_id, \"name\", value, value_hex) DO NOTHING")
                                .bind(&id_blob)
                                .bind(tag_name)
                                .bind(tag_val.as_bytes())
                                .execute(&mut *tx)
                                .await?;
                        }
                    }
                    None => {}
                }
            }
        }
        if e.is_replaceable() {
            let update_count = sqlx::query("DELETE FROM \"event\" WHERE kind=$1 and pub_key = $2 and id not in (select id from \"event\" where kind=$1 and pub_key=$2 order by created_at desc limit 1);")
                .bind(e.kind as i64)
                .bind(hex::decode(&e.pubkey).ok())
                .execute(&mut *tx)
                .await?.rows_affected();
            if update_count > 0 {
                info!(
                    "hid {} older replaceable kind {} events for author: {:?}",
                    update_count,
                    e.kind,
                    e.get_author_prefix()
                );
            }
        }
        // parameterized replaceable events
        // check for parameterized replaceable events that would be hidden; don't insert these either.
        if let Some(d_tag) = e.distinct_param() {
            let update_count = if is_lower_hex(&d_tag) && (d_tag.len() % 2 == 0) {
                sqlx::query("DELETE FROM event WHERE kind=$1 AND pub_key=$2 AND id IN (SELECT e.id FROM event e LEFT JOIN tag t ON e.id=t.event_id WHERE e.kind=$1 AND e.pub_key=$2 AND t.name='d' AND t.value_hex=$3 ORDER BY created_at DESC OFFSET 1);")
                    .bind(e.kind as i64)
                    .bind(hex::decode(&e.pubkey).ok())
                    .bind(hex::decode(d_tag).ok())
                    .execute(&mut *tx)
                    .await?.rows_affected()
            } else {
                sqlx::query("DELETE FROM event WHERE kind=$1 AND pub_key=$2 AND id IN (SELECT e.id FROM event e LEFT JOIN tag t ON e.id=t.event_id WHERE e.kind=$1 AND e.pub_key=$2 AND t.name='d' AND t.value=$3 ORDER BY created_at DESC OFFSET 1);")
                    .bind(e.kind as i64)
                    .bind(hex::decode(&e.pubkey).ok())
                    .bind(d_tag.as_bytes())
                    .execute(&mut *tx)
                    .await?.rows_affected()
            };
            if update_count > 0 {
                info!(
                    "removed {} older parameterized replaceable kind {} events for author: {:?}",
                    update_count,
                    e.kind,
                    e.get_author_prefix()
                );
            }
        }
        // if this event is a deletion, hide the referenced events from the same author.
        if e.kind == 5 {
            let event_candidates = e.tag_values_by_name("e");
            let pub_keys: Vec<Vec<u8>> = event_candidates
                .iter()
                .filter(|x| is_hex(x) && x.len() == 64)
                .filter_map(|x| hex::decode(x).ok())
                .collect();

            let mut builder = QueryBuilder::new(
                "UPDATE \"event\" SET hidden = 1::bit(1) WHERE kind != 5 AND pub_key = ",
            );
            builder.push_bind(hex::decode(&e.pubkey).ok());
            builder.push(" AND id IN (");

            let mut sep = builder.separated(", ");
            for pk in pub_keys {
                sep.push_bind(pk);
            }
            sep.push_unseparated(")");

            let update_count = builder.build().execute(&mut *tx).await?.rows_affected();
            info!(
                "hid {} deleted events for author {:?}",
                update_count,
                e.get_author_prefix()
            );
        } else {
            // check if a deletion has already been recorded for this event.
            // Only relevant for non-deletion events
            let del_count = sqlx::query(
                "SELECT e.id FROM \"event\" e \
            LEFT JOIN tag t ON e.id = t.event_id \
            WHERE e.pub_key = $1 AND t.\"name\" = 'e' AND e.kind = 5 AND t.value = $2 LIMIT 1",
            )
            .bind(&pubkey_blob)
            .bind(&id_blob)
            .fetch_optional(&mut *tx)
            .await?;

            // check if a the query returned a result, meaning we should
            // hid the current event
            if del_count.is_some() {
                // a deletion already existed, mark original event as hidden.
                info!(
                    "hid event: {:?} due to existing deletion by author: {:?}",
                    e.get_event_id_prefix(),
                    e.get_author_prefix()
                );
                sqlx::query("UPDATE \"event\" SET hidden = 1::bit(1) WHERE id = $1")
                    .bind(&id_blob)
                    .execute(&mut *tx)
                    .await?;
                // event was deleted, so let caller know nothing new
                // arrived, preventing this from being sent to active
                // subscriptions
                ins_count = 0;
            }
        }
        tx.commit().await?;
        self.metrics
            .write_events
            .observe(start.elapsed().as_secs_f64());
        Ok(ins_count)
    }
}
