use crate::config::Settings;
use governor::{clock::Clock, clock::QuantaClock, Quota, RateLimiter as Governor};
use std::time::{Duration, Instant};
use tracing::warn;

pub struct RateLimiter {
    limiter: Option<Governor<QuantaClock>>,
    most_recent_rate_limit: Instant,
    clock: QuantaClock,
}

impl RateLimiter {
    pub fn new(settings: &Settings) -> Self {
        let mut limiter = None;
        if let Some(rps) = settings.limits.messages_per_sec {
            if rps > 0 {
                let quota = core::num::NonZeroU32::new(rps * 60).unwrap();
                limiter = Some(Governor::direct(Quota::per_minute(quota)));
            }
        }

        Self {
            limiter,
            most_recent_rate_limit: Instant::now(),
            clock: QuantaClock::default(),
        }
    }

    pub fn check_rate_limit(&mut self) -> Option<Duration> {
        if let Some(ref lim) = self.limiter {
            if let Err(n) = lim.check() {
                let wait_for = n.wait_time_from(self.clock.now());

                // Log rate limit message once per 10 seconds
                if self.most_recent_rate_limit.elapsed().as_secs() > 10 {
                    warn!(
                        "rate limit reached for event creation (sleep for {:?}) (suppressing future messages for 10 seconds)",
                        wait_for
                    );
                    self.most_recent_rate_limit = Instant::now();
                }

                return Some(wait_for);
            }
        }
        None
    }
}
