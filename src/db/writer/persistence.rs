use crate::error::Result;
use crate::event::Event;
use crate::repo::NostrRepo;
use crate::db::writer::SubmittedEvent;
use std::sync::Arc;
use tokio::sync::broadcast::Sender;
use tracing::{debug, trace};

pub struct EventPersistence {
    repo: Arc<dyn NostrRepo>,
    bcast_tx: Sender<Event>,
    metadata_tx: Sender<Event>,
}

impl EventPersistence {
    pub fn new(
        repo: Arc<dyn NostrRepo>,
        bcast_tx: Sender<Event>,
        metadata_tx: Sender<Event>,
    ) -> Self {
        Self {
            repo,
            bcast_tx,
            metadata_tx,
        }
    }

    pub async fn persist_event(&self, event: &Event, subm_event: &SubmittedEvent) -> Result<bool> {
        // Handle metadata events
        if event.is_kind_metadata() {
            self.metadata_tx.send(event.clone()).ok();
        }

        // Handle ephemeral events
        if event.is_ephemeral() {
            self.bcast_tx.send(event.clone()).ok();
            debug!(
                "published ephemeral event: {:?} from: {:?}",
                event.get_event_id_prefix(),
                event.get_author_prefix(),
            );
            return Ok(true);
        }

        // Persist regular events
        match self.repo.write_event(event).await {
            Ok(updated) => {
                if updated == 0 {
                    trace!("ignoring duplicate or deleted event");
                    Ok(false)
                } else {
                    debug!(
                        "persisted event: {:?} (kind: {}) from: {:?} (IP: {:?})",
                        event.get_event_id_prefix(),
                        event.kind,
                        event.get_author_prefix(),
                        subm_event.source_ip,
                    );
                    self.bcast_tx.send(event.clone()).ok();
                    Ok(true)
                }
            }
            Err(e) => Err(e),
        }
    }
}