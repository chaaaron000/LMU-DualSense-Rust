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
                    if telemetry_log.ready() {
                        info!(
                            connected = frame.connected,
                            vehicle = frame.player_has_vehicle,
                            throttle = format_args!("{:.2}", frame.throttle),
                            brake = format_args!("{:.2}", frame.brake),
                            rpm = format_args!("{:.0}/{:.0}", frame.rpm, frame.max_rpm),
                            gear = frame.gear,
                            abs = frame.abs_active,
                            tc = frame.tc_active,
                            "telemetry"
                        );
                    }

                    let trigger_frame = self.mapper.map(frame);
                    if let Err(error) = self.output.send(&trigger_frame) {
                        warn!(%error, "trigger output failed");
                    }
                }
                Err(error) => warn!(%error, "telemetry poll failed"),
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
