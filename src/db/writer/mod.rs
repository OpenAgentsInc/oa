use crate::config::Settings;
use crate::error::{Error, Result};
use crate::event::Event;
use crate::notice::Notice;
use crate::payment::PaymentMessage;
use crate::repo::NostrRepo;
use std::sync::Arc;
use std::time::Instant;
use tracing::{debug, info};

mod payment;
mod persistence;
mod rate_limit;
mod validation;

use payment::PaymentHandler;
use persistence::EventPersistence;
use rate_limit::RateLimiter;
use validation::EventValidator;

/// Events submitted from a client, with a return channel for notices
pub struct SubmittedEvent {
    pub event: Event,
    pub notice_tx: tokio::sync::mpsc::Sender<Notice>,
    pub source_ip: String,
    pub origin: Option<String>,
    pub user_agent: Option<String>,
    pub auth_pubkey: Option<Vec<u8>>,
}

/// Spawn a database writer that persists events to the store.
pub async fn db_writer(
    repo: Arc<dyn NostrRepo>,
    settings: Settings,
    mut event_rx: tokio::sync::mpsc::Receiver<SubmittedEvent>,
    bcast_tx: tokio::sync::broadcast::Sender<Event>,
    metadata_tx: tokio::sync::broadcast::Sender<Event>,
    payment_tx: tokio::sync::broadcast::Sender<PaymentMessage>,
    mut shutdown: tokio::sync::broadcast::Receiver<()>,
) -> Result<()> {
    let validator = EventValidator::new(&settings);
    let payment_handler = PaymentHandler::new(repo.clone(), &settings);
    let persistence = EventPersistence::new(repo.clone(), bcast_tx, metadata_tx);
    let rate_limiter = RateLimiter::new(&settings);

    loop {
        if shutdown.try_recv().is_ok() {
            info!("shutting down database writer");
            break;
        }

        // call blocking read on channel
        let next_event = event_rx.recv().await;
        // if the channel has closed, we will never get work
        if next_event.is_none() {
            break;
        }

        let start = Instant::now();
        let subm_event = next_event.unwrap();
        let event = subm_event.event;
        let notice_tx = subm_event.notice_tx;

        // Validate event
        if let Err(notice) = validator.validate_event(&event).await {
            notice_tx.try_send(notice).ok();
            continue;
        }

        // Check payment if required
        if let Err(notice) = payment_handler.check_payment(&event, &payment_tx).await {
            notice_tx.try_send(notice).ok();
            continue;
        }

        // Persist event
        match persistence.persist_event(&event, &subm_event).await {
            Ok(true) => {
                debug!(
                    "processed event: {:?} from: {:?} in: {:?}",
                    event.get_event_id_prefix(),
                    event.get_author_prefix(),
                    start.elapsed()
                );

                // Update payment balance if needed
                if let Err(e) = payment_handler.update_balance(&event).await {
                    debug!("Failed to update balance: {:?}", e);
                }

                // Apply rate limiting
                if let Some(wait_time) = rate_limiter.check_rate_limit() {
                    std::thread::sleep(wait_time);
                }

                notice_tx.try_send(Notice::saved(event.id)).ok();
            }
            Ok(false) => {
                notice_tx.try_send(Notice::duplicate(event.id)).ok();
            }
            Err(e) => {
                debug!("Event persistence failed: {:?}", e);
                notice_tx
                    .try_send(Notice::error(
                        event.id,
                        "relay experienced an error trying to publish the event",
                    ))
                    .ok();
            }
        }
    }

    info!("database connection closed");
    Ok(())
}
