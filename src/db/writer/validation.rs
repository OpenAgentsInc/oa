use crate::config::Settings;
use crate::error::Result;
use crate::event::Event;
use crate::notice::Notice;
use tracing::debug;

pub struct EventValidator {
    kinds_blacklist: Option<Vec<i64>>,
    kinds_allowlist: Option<Vec<i64>>,
    nip05_enabled: bool,
}

impl EventValidator {
    pub fn new(settings: &Settings) -> Self {
        Self {
            kinds_blacklist: settings.limits.event_kind_blacklist.clone(),
            kinds_allowlist: settings.limits.event_kind_allowlist.clone(),
            nip05_enabled: settings.verified_users.is_enabled(),
        }
    }

    pub async fn validate_event(&self, event: &Event) -> Result<(), Notice> {
        // Check blacklist
        if let Some(ref blacklist) = self.kinds_blacklist {
            if blacklist.contains(&event.kind) {
                debug!(
                    "rejecting event: {}, blacklisted kind: {}",
                    &event.get_event_id_prefix(),
                    &event.kind
                );
                return Err(Notice::blocked(
                    event.id,
                    "event kind is blocked by relay",
                ));
            }
        }

        // Check allowlist
        if let Some(ref allowlist) = self.kinds_allowlist {
            if !allowlist.contains(&event.kind) {
                debug!(
                    "rejecting event: {}, allowlist kind: {}",
                    &event.get_event_id_prefix(),
                    &event.kind
                );
                return Err(Notice::blocked(
                    event.id,
                    "event kind is blocked by relay",
                ));
            }
        }

        Ok(())
    }
}