use std::time::{Duration, Instant};

use anyhow::Result;
use tracing::{info, warn};

use crate::config::LmuConfig;

#[cfg(windows)]
use super::windows_shared_memory::LmuSharedMemory;
use super::{lmu_layout, TelemetryFrame, TelemetryReader};

const RECONNECT_INTERVAL: Duration = Duration::from_secs(2);

pub struct LmuSharedMemoryReader {
    config: LmuConfig,
    #[cfg(windows)]
    connection: Option<LmuSharedMemory>,
    snapshot: Vec<u8>,
    wheel_lock: lmu_layout::WheelLockEstimator,
    next_reconnect: Instant,
    connected: bool,
    player_has_vehicle: bool,
    waiting_logged: bool,
}

impl LmuSharedMemoryReader {
    pub fn new(config: LmuConfig) -> Self {
        info!(
            "[LMU] Waiting for Le Mans Ultimate | map={}",
            config.shared_memory_name
        );
        Self {
            config,
            #[cfg(windows)]
            connection: None,
            snapshot: vec![0; lmu_layout::SNAPSHOT_SIZE],
            wheel_lock: lmu_layout::WheelLockEstimator::default(),
            next_reconnect: Instant::now(),
            connected: false,
            player_has_vehicle: false,
            waiting_logged: true,
        }
    }

    #[cfg(windows)]
    fn connect_if_due(&mut self) {
        if self.connection.is_some() || Instant::now() < self.next_reconnect {
            return;
        }

        match LmuSharedMemory::open(&self.config.shared_memory_name) {
            Ok(connection) => {
                info!(
                    "[LMU] Shared memory connected | map={}",
                    self.config.shared_memory_name
                );
                self.connection = Some(connection);
                self.waiting_logged = false;
            }
            Err(error) => {
                if !self.waiting_logged {
                    info!("[LMU] Waiting to reconnect | {error}");
                    self.waiting_logged = true;
                }
                self.next_reconnect = Instant::now() + RECONNECT_INTERVAL;
            }
        }
    }

    fn update_state(&mut self, frame: TelemetryFrame) {
        if frame.connected != self.connected {
            if frame.connected {
                info!("[LMU] Telemetry active");
            } else if self.connected {
                warn!("[LMU] Telemetry disconnected | Triggers reset");
            }
            self.connected = frame.connected;
        }

        if frame.player_has_vehicle != self.player_has_vehicle {
            if frame.player_has_vehicle {
                info!("[LMU] Player vehicle acquired");
            } else if self.player_has_vehicle {
                info!("[LMU] Player vehicle released");
            }
            self.player_has_vehicle = frame.player_has_vehicle;
        }
    }

    #[cfg(windows)]
    fn disconnect(&mut self, error: &anyhow::Error) {
        warn!("[LMU] Connection lost | {error}");
        self.connection = None;
        self.wheel_lock.reset();
        self.next_reconnect = Instant::now() + RECONNECT_INTERVAL;
        self.waiting_logged = false;
        self.update_state(TelemetryFrame::default());
    }
}

impl TelemetryReader for LmuSharedMemoryReader {
    fn poll(&mut self) -> Result<TelemetryFrame> {
        #[cfg(not(windows))]
        {
            let _ = &self.config;
            return Ok(TelemetryFrame::default());
        }

        #[cfg(windows)]
        {
            self.connect_if_due();
            let Some(connection) = self.connection.as_ref() else {
                return Ok(TelemetryFrame::default());
            };

            match connection.is_process_alive() {
                Ok(true) => {}
                Ok(false) => {
                    self.disconnect(&anyhow::anyhow!("Le Mans Ultimate exited"));
                    return Ok(TelemetryFrame::default());
                }
                Err(error) => {
                    self.disconnect(&error);
                    return Ok(TelemetryFrame::default());
                }
            }

            if let Err(error) = connection.copy_snapshot(&mut self.snapshot) {
                self.disconnect(&error);
                return Ok(TelemetryFrame::default());
            }

            match lmu_layout::parse_snapshot(&self.snapshot, &mut self.wheel_lock) {
                Ok(frame) => {
                    self.update_state(frame);
                    Ok(frame)
                }
                Err(error) => {
                    self.disconnect(&error.context("LMU layout validation failed"));
                    Ok(TelemetryFrame::default())
                }
            }
        }
    }
}
