use anyhow::Result;
use tracing::warn;

use crate::config::LmuConfig;

use super::{TelemetryFrame, TelemetryReader};

pub struct LmuSharedMemoryReader {
    config: LmuConfig,
}

impl LmuSharedMemoryReader {
    pub fn new(config: LmuConfig) -> Self {
        warn!(
            name = %config.shared_memory_name,
            "LMU shared-memory reader is a v0.1 scaffold; returning disconnected telemetry"
        );
        Self { config }
    }
}

impl TelemetryReader for LmuSharedMemoryReader {
    fn poll(&mut self) -> Result<TelemetryFrame> {
        let _shared_memory_name = &self.config.shared_memory_name;
        // v0.2 TODO:
        // 1. Open the configured Windows named mapping.
        // 2. Validate the mapped byte length against the LMU layout.
        // 3. Validate playerHasVehicle/playerVehicleIdx.
        // 4. Copy only the required fields into TelemetryFrame.
        Ok(TelemetryFrame::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scaffold_returns_disconnected_frame() {
        let mut reader = LmuSharedMemoryReader::new(LmuConfig::default());
        let frame = reader.poll().unwrap();
        assert!(!frame.connected);
        assert!(!frame.player_has_vehicle);
    }
}
