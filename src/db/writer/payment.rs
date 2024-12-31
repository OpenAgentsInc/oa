use crate::config::Settings;
use crate::error::{Error, Result};
use crate::event::Event;
use crate::notice::Notice;
use crate::payment::PaymentMessage;
use crate::repo::NostrRepo;
use nostr::key::{FromPkStr, Keys};
use std::sync::Arc;
use tokio::sync::broadcast::Sender;
use tracing::{debug, info};

pub struct PaymentHandler {
    repo: Arc<dyn NostrRepo>,
    enabled: bool,
    cost_per_event: u64,
    whitelist: Option<Vec<Vec<u8>>>,
    sign_ups: bool,
    direct_message: bool,
}

impl PaymentHandler {
    pub fn new(repo: Arc<dyn NostrRepo>, settings: &Settings) -> Self {
        Self {
            repo,
            enabled: settings.pay_to_relay.enabled,
            cost_per_event: settings.pay_to_relay.cost_per_event,
            whitelist: settings.authorization.pubkey_whitelist.clone(),
            sign_ups: settings.pay_to_relay.sign_ups,
            direct_message: settings.pay_to_relay.direct_message,
        }
    }

    pub async fn check_payment(
        &self,
        event: &Event,
        payment_tx: &Sender<PaymentMessage>,
    ) -> Result<(), Notice> {
        if !self.enabled {
            return Ok(());
        }

        // Check whitelist first
        if let Some(ref whitelist) = self.whitelist {
            if whitelist.contains(&event.pubkey) {
                return Ok(());
            }
        }

        let key = Keys::from_pk_str(&event.pubkey)
            .map_err(|_| Notice::error(event.id, "invalid pubkey"))?;

        match self.repo.get_account_balance(&key).await {
            Ok((user_admitted, balance)) => {
                if !user_admitted {
                    debug!("user: {}, is not admitted", &event.pubkey);
                    payment_tx
                        .send(PaymentMessage::CheckAccount(event.pubkey.clone()))
                        .ok();
                    return Err(Notice::blocked(event.id, "User is not admitted"));
                }

                if balance < self.cost_per_event {
                    debug!("user: {}, does not have a balance", &event.pubkey);
                    return Err(Notice::blocked(event.id, "Insufficient balance"));
                }
            }
            Err(Error::SqlxError(sqlx::Error::RowNotFound)) => {
                info!("Unregistered user");
                if self.sign_ups && self.direct_message {
                    payment_tx
                        .send(PaymentMessage::NewAccount(event.pubkey.clone()))
                        .ok();
                }
                return Err(Notice::error(event.id, "Pubkey not registered"));
            }
            Err(err) => {
                debug!("Error checking admission status: {:?}", err);
                return Err(Notice::error(
                    event.id,
                    "relay experienced an error checking your admission status",
                ));
            }
        }

        Ok(())
    }

    pub async fn update_balance(&self, event: &Event) -> Result<()> {
        if !self.enabled || self.cost_per_event == 0 {
            return Ok(());
        }

        // Skip whitelisted users
        if let Some(ref whitelist) = self.whitelist {
            if whitelist.contains(&event.pubkey) {
                return Ok(());
            }
        }

        let pubkey = Keys::from_pk_str(&event.pubkey)?;
        self.repo
            .update_account_balance(&pubkey, false, self.cost_per_event)
            .await
    }
}
