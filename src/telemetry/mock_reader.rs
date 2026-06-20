use anyhow::Result;

use super::{TelemetryFrame, TelemetryReader};

const SCENARIO_COUNT: u64 = 7;

pub struct MockTelemetryReader {
    tick_hz: u32,
    tick: u64,
}

impl MockTelemetryReader {
    pub fn new(tick_hz: u32) -> Self {
        Self { tick_hz, tick: 0 }
    }

    fn frame_at(&self, tick: u64) -> TelemetryFrame {
        let ticks_per_stage = u64::from(self.tick_hz.max(1));
        let stage = (tick / ticks_per_stage) % SCENARIO_COUNT;
        let stage_tick = tick % ticks_per_stage;
        let progress = stage_tick as f32 / ticks_per_stage as f32;

        let mut frame = TelemetryFrame {
            connected: true,
            player_has_vehicle: true,
            rpm: 2_000.0,
            max_rpm: 8_000.0,
            gear: 2,
            speed_mps: 25.0,
            abs_level: 3,
            tc_level: 4,
            tc_slip_level: 2,
            tc_cut_level: 2,
            ..TelemetryFrame::default()
        };

        match stage {
            0 => {}
            1 => frame.brake = progress,
            2 => {
                frame.brake = 0.9;
                frame.abs_active = true;
            }
            3 => frame.throttle = progress,
            4 => {
                frame.throttle = 0.8;
                frame.tc_active = true;
            }
            5 => {
                frame.throttle = 1.0;
                frame.rpm = frame.max_rpm * (0.95 + progress * 0.05);
            }
            6 => {
                frame.throttle = 0.6;
                frame.gear = if progress < 0.5 { 3 } else { 4 };
                frame.rpm = if progress < 0.5 { 7_200.0 } else { 5_200.0 };
            }
            _ => unreachable!(),
        }

        frame
    }
}

impl TelemetryReader for MockTelemetryReader {
    fn poll(&mut self) -> Result<TelemetryFrame> {
        let frame = self.frame_at(self.tick);
        self.tick = self.tick.wrapping_add(1);
        Ok(frame)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn produces_each_mock_scenario() {
        let reader = MockTelemetryReader::new(10);

        assert_eq!(reader.frame_at(0).brake, 0.0);
        assert!(reader.frame_at(15).brake > 0.0);
        assert!(reader.frame_at(20).abs_active);
        assert!(reader.frame_at(35).throttle > 0.0);
        assert!(reader.frame_at(40).tc_active);
        assert!(reader.frame_at(59).rpm > 7_900.0);
        assert_eq!(reader.frame_at(69).gear, 4);
    }
}
