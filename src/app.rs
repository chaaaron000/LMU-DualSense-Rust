use std::{
    thread,
    time::{Duration, Instant},
};

use anyhow::Result;
use tracing::{info, warn};

use crate::{
    effects::EffectMapper, output::TriggerOutput, telemetry::TelemetryReader, util::RateLimiter,
};

pub struct App {
    reader: Box<dyn TelemetryReader>,
    mapper: EffectMapper,
    output: Box<dyn TriggerOutput>,
    tick_hz: u32,
}

impl App {
    pub fn new(
        reader: Box<dyn TelemetryReader>,
        mapper: EffectMapper,
        output: Box<dyn TriggerOutput>,
        tick_hz: u32,
    ) -> Self {
        Self {
            reader,
            mapper,
            output,
            tick_hz,
        }
    }

    pub fn run(mut self) -> Result<()> {
        let tick_interval = Duration::from_secs_f64(1.0 / f64::from(self.tick_hz));
        let mut next_tick = Instant::now();
        let mut telemetry_log = RateLimiter::per_second();

        loop {
            next_tick += tick_interval;

            match self.reader.poll() {
                Ok(frame) => {
                    let trigger_frame = self.mapper.map(frame);
                    if telemetry_log.ready() && frame.connected {
                        if frame.player_has_vehicle {
                            info!(
                                "[LIVE] G {:>2} | {:>3.0} km/h | RPM {:>5.0}/{:<5.0} | THR {:>3.0}% | BRK {:>3.0}% | ABS {} | TC {} | L2 {} | R2 {}",
                                frame.gear,
                                frame.speed_mps * 3.6,
                                frame.rpm,
                                frame.max_rpm,
                                frame.throttle * 100.0,
                                frame.brake * 100.0,
                                active_label(frame.abs_active),
                                active_label(frame.tc_active),
                                trigger_frame.left,
                                trigger_frame.right,
                            );
                        } else {
                            info!("[LIVE] LMU connected | Waiting for player vehicle");
                        }
                    }

                    if let Err(error) = self.output.send(&trigger_frame) {
                        warn!("[OUTPUT] Send failed | {error}");
                    }
                }
                Err(error) => warn!("[TELEMETRY] Poll failed | {error}"),
            }

            let now = Instant::now();
            if next_tick > now {
                thread::sleep(next_tick - now);
            } else {
                next_tick = now;
            }
        }
    }
}

fn active_label(active: bool) -> &'static str {
    if active {
        "ON "
    } else {
        "OFF"
    }
}
