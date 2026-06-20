use anyhow::Result;
use tracing::info;

use crate::{effects::TriggerOutputFrame, util::RateLimiter};

use super::TriggerOutput;

pub struct NullOutput {
    log_limiter: RateLimiter,
}

impl Default for NullOutput {
    fn default() -> Self {
        Self {
            log_limiter: RateLimiter::per_second(),
        }
    }
}

impl TriggerOutput for NullOutput {
    fn send(&mut self, frame: &TriggerOutputFrame) -> Result<()> {
        if self.log_limiter.ready() {
            info!(left = ?frame.left, right = ?frame.right, "null output");
        }
        Ok(())
    }
}
