use anyhow::Result;

use super::TelemetryFrame;

pub trait TelemetryReader {
    fn poll(&mut self) -> Result<TelemetryFrame>;
}
