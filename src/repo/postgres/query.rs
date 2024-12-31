use crate::db::QueryResult;
use crate::error::Result;
use crate::subscription::{ReqFilter, Subscription};
use crate::utils::{is_hex, is_lower_hex};
use async_std::stream::StreamExt;
use chrono::Utc;
use sqlx::{Postgres, QueryBuilder};
use std::time::{Duration, Instant};
use tokio::sync::mpsc::Sender;
use tokio::sync::oneshot::Receiver;
use tracing::{debug, error, info, trace};

impl super::PostgresRepo {
    pub(crate) async fn query_subscription(
        &self,
        sub: Subscription,
        client_id: String,
        query_tx: Sender<QueryResult>,
        mut abandon_query_rx: Receiver<()>,
    ) -> Result<()> {
        let start = Instant::now();
        let mut row_count: usize = 0;
        let metrics = &self.metrics;

        for filter in sub.filters.iter() {
            let start = Instant::now();
            // generate SQL query
            let q_filter = query_from_filter(filter);
            if q_filter.is_none() {
                debug!("Failed to generate query!");
                continue;
            }

            debug!("SQL generated in {:?}", start.elapsed());

            // cutoff for displaying slow queries
            let slow_cutoff = Duration::from_millis(2000);

            // any client that doesn't cause us to generate new rows in 5
            // seconds gets dropped.
            let abort_cutoff = Duration::from_secs(5);

            let start = Instant::now();
            let mut slow_first_event;
            let mut last_successful_send = Instant::now();

            // execute the query. Don't cache, since queries vary so much.
            let mut q_filter = q_filter.unwrap();
            let q_build = q_filter.build();
            let sql = q_build.sql();
            let mut results = q_build.fetch(&self.conn);

            let mut first_result = true;
            while let Some(row) = results.next().await {
                if let Err(e) = row {
                    error!("Query failed: {} {} {:?}", e, sql, filter);
                    break;
                }
                let first_event_elapsed = start.elapsed();
                slow_first_event = first_event_elapsed >= slow_cutoff;
                if first_result {
                    debug!(
                        "first result in {:?} (cid: {}, sub: {:?})",
                        first_event_elapsed, client_id, sub.id
                    );
                    first_result = false;
                }

                // logging for slow queries; show sub and SQL.
                // to reduce logging; only show 1/16th of clients (leading 0)
                if slow_first_event && client_id.starts_with("00") {
                    debug!(
                        "query req (slow): {:?} (cid: {}, sub: {:?})",
                        &sub, client_id, sub.id
                    );
                } else {
                    trace!(
                        "query req: {:?} (cid: {}, sub: {:?})",
                        &sub,
                        client_id,
                        sub.id
                    );
                }

                // check if this is still active; every 100 rows
                if row_count % 100 == 0 && abandon_query_rx.try_recv().is_ok() {
                    debug!(
                        "query cancelled by client (cid: {}, sub: {:?})",
                        client_id, sub.id
                    );
                    return Ok(());
                }

                row_count += 1;
                let event_json: Vec<u8> = row.unwrap().get(0);
                loop {
                    if query_tx.capacity() != 0 {
                        // we have capacity to add another item
                        break;
                    } else {
                        // the queue is full
                        trace!("db reader thread is stalled");
                        if last_successful_send + abort_cutoff < Instant::now() {
                            // the queue has been full for too long, abort
                            info!("aborting database query due to slow client");
                            metrics
                                .query_aborts
                                .with_label_values(&["slowclient"])
                                .inc();
                            return Ok(());
                        }
                        // give the queue a chance to clear before trying again
                        async_std::task::sleep(Duration::from_millis(100)).await;
                    }
                }

                query_tx
                    .send(QueryResult {
                        sub_id: sub.get_id(),
                        event: String::from_utf8(event_json).unwrap(),
                    })
                    .await
                    .ok();
                last_successful_send = Instant::now();
            }
        }
        query_tx
            .send(QueryResult {
                sub_id: sub.get_id(),
                event: "EOSE".to_string(),
            })
            .await
            .ok();
        self.metrics
            .query_sub
            .observe(start.elapsed().as_secs_f64());
        debug!(
            "query completed in {:?} (cid: {}, sub: {:?}, db_time: {:?}, rows: {})",
            start.elapsed(),
            client_id,
            sub.id,
            start.elapsed(),
            row_count
        );
        Ok(())
    }
}

/// Create a dynamic SQL query and params from a subscription filter.
fn query_from_filter(f: &ReqFilter) -> Option<QueryBuilder<Postgres>> {
    // if the filter is malformed, don't return anything.
    if f.force_no_match {
        return None;
    }

    let mut query = QueryBuilder::new("SELECT e.\"content\", e.created_at FROM \"event\" e WHERE ");

    // This tracks whether we need to push a prefix AND before adding another clause
    let mut push_and = false;
    // Query for "authors", allowing prefix matches
    if let Some(auth_vec) = &f.authors {
        // filter out non-hex values
        let auth_vec: Vec<&String> = auth_vec.iter().filter(|a| is_hex(a)).collect();

        if auth_vec.is_empty() {
            return None;
        }
        query.push("(e.pub_key in (");

        let mut pk_sep = query.separated(", ");
        for pk in auth_vec.iter() {
            pk_sep.push_bind(hex::decode(pk).ok());
        }
        query.push(") OR e.delegated_by in (");
        let mut pk_delegated_sep = query.separated(", ");
        for pk in auth_vec.iter() {
            pk_delegated_sep.push_bind(hex::decode(pk).ok());
        }
        push_and = true;
        query.push("))");
    }

    // Query for Kind
    if let Some(ks) = &f.kinds {
        if ks.is_empty() {
            return None;
        }
        if push_and {
            query.push(" AND ");
        }
        push_and = true;

        query.push("e.kind in (");
        let mut list_query = query.separated(", ");
        for k in ks.iter() {
            list_query.push_bind(*k as i64);
        }
        query.push(")");
    }

    // Query for event,
    if let Some(id_vec) = &f.ids {
        // filter out non-hex values
        let id_vec: Vec<&String> = id_vec.iter().filter(|a| is_hex(a)).collect();
        if id_vec.is_empty() {
            return None;
        }
        if push_and {
            query.push(" AND (");
        } else {
            query.push("(");
        }
        push_and = true;

        query.push("id in (");
        let mut sep = query.separated(", ");
        for id in id_vec.iter() {
            sep.push_bind(hex::decode(id).ok());
        }
        query.push("))");
    }

    // Query for tags
    if let Some(map) = &f.tags {
        if !map.is_empty() {
            if push_and {
                query.push(" AND ");
            }
            push_and = true;

            let mut push_or = false;
            query.push("e.id IN (SELECT ee.id FROM \"event\" ee LEFT JOIN tag t on ee.id = t.event_id WHERE ee.hidden != 1::bit(1) and ");
            for (key, val) in map.iter() {
                if val.is_empty() {
                    return None;
                }
                if push_or {
                    query.push(" OR ");
                }
                query
                    .push("(t.\"name\" = ")
                    .push_bind(key.to_string())
                    .push(" AND (");

                let has_plain_values = val.iter().any(|v| (v.len() % 2 != 0 || !is_lower_hex(v)));
                let has_hex_values = val.iter().any(|v| v.len() % 2 == 0 && is_lower_hex(v));
                if has_plain_values {
                    query.push("value in (");
                    // plain value match first
                    let mut tag_query = query.separated(", ");
                    for v in val.iter().filter(|v| v.len() % 2 != 0 || !is_lower_hex(v)) {
                        tag_query.push_bind(v.as_bytes());
                    }
                }
                if has_plain_values && has_hex_values {
                    query.push(") OR ");
                }
                if has_hex_values {
                    query.push("value_hex in (");
                    // plain value match first
                    let mut tag_query = query.separated(", ");
                    for v in val.iter().filter(|v| v.len() % 2 == 0 && is_lower_hex(v)) {
                        tag_query.push_bind(hex::decode(v).ok());
                    }
                }

                query.push(")))");
                push_or = true;
            }
            query.push(")");
        }
    }

    // Query for timestamp
    if f.since.is_some() {
        if push_and {
            query.push(" AND ");
        }
        push_and = true;
        query
            .push("e.created_at >= ")
            .push_bind(Utc.timestamp_opt(f.since.unwrap() as i64, 0).unwrap());
    }

    // Query for timestamp
    if f.until.is_some() {
        if push_and {
            query.push(" AND ");
        }
        push_and = true;
        query
            .push("e.created_at <= ")
            .push_bind(Utc.timestamp_opt(f.until.unwrap() as i64, 0).unwrap());
    }

    // never display hidden events
    if push_and {
        query.push(" AND e.hidden != 1::bit(1)");
    } else {
        query.push("e.hidden != 1::bit(1)");
    }
    // never display expired events
    query.push(" AND (e.expires_at IS NULL OR e.expires_at > now())");

    // Apply per-filter limit to this query.
    // The use of a LIMIT implies a DESC order, to capture only the most recent events.
    if let Some(lim) = f.limit {
        query.push(" ORDER BY e.created_at DESC LIMIT ");
        query.push(lim.min(1000));
    } else {
        query.push(" ORDER BY e.created_at ASC LIMIT ");
        query.push(1000);
    }
    Some(query)
}
