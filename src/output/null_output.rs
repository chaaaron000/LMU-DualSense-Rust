use anyhow::Result;

use crate::effects::TriggerOutputFrame;

use super::TriggerOutput;

#[derive(Default)]
pub struct NullOutput;

impl TriggerOutput for NullOutput {
    fn send(&mut self, _frame: &TriggerOutputFrame) -> Result<()> {
        Ok(())
    }
}
