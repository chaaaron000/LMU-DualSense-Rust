mod dsx_udp;
mod null_output;

use anyhow::Result;

pub use dsx_udp::{DsxPacketEncoder, DsxUdpOutput};
pub use null_output::NullOutput;

use crate::effects::TriggerOutputFrame;

pub trait TriggerOutput {
    fn send(&mut self, frame: &TriggerOutputFrame) -> Result<()>;
}
