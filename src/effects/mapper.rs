use crate::{
    config::{BrakeEffectConfig, EffectConfig, SmoothingConfig, ThrottleEffectConfig},
    telemetry::TelemetryFrame,
};

use super::{Smoother, TriggerEffect, TriggerOutputFrame};

pub struct EffectMapper {
    config: EffectConfig,
    smoothing: SmoothingConfig,
    brake_smoother: Smoother,
    throttle_smoother: Smoother,
}

impl EffectMapper {
    pub fn new(config: EffectConfig, smoothing: SmoothingConfig) -> Self {
        Self {
            brake_smoother: Smoother::new(smoothing.attack, smoothing.release),
            throttle_smoother: Smoother::new(smoothing.attack, smoothing.release),
            config,
            smoothing,
        }
    }

    pub fn map(&mut self, frame: TelemetryFrame) -> TriggerOutputFrame {
        if !frame.connected || !frame.player_has_vehicle {
            self.brake_smoother.reset();
            self.throttle_smoother.reset();
            return TriggerOutputFrame {
                left: TriggerEffect::Normal,
                right: TriggerEffect::Normal,
            };
        }

        let brake = if self.smoothing.enabled {
            self.brake_smoother.update(frame.brake)
        } else {
            frame.brake.clamp(0.0, 1.0)
        };
        let throttle = if self.smoothing.enabled {
            self.throttle_smoother.update(frame.throttle)
        } else {
            frame.throttle.clamp(0.0, 1.0)
        };

        TriggerOutputFrame {
            left: map_brake(brake, frame.abs_active, &self.config.brake),
            right: self.map_throttle(frame, throttle),
        }
    }

    fn map_throttle(&self, frame: TelemetryFrame, throttle: f32) -> TriggerEffect {
        let rpm_ratio = if frame.max_rpm > 0.0 {
            frame.rpm / frame.max_rpm
        } else {
            0.0
        };

        if self.config.rpm.enabled && rpm_ratio >= self.config.rpm.rev_limit_ratio {
            return TriggerEffect::Vibrate {
                start: self.config.throttle.start_position.min(10),
                force: self.config.rpm.vibration_force.min(10),
                frequency: self.config.rpm.vibration_frequency.min(10),
            };
        }

        map_throttle(throttle, frame.tc_active, &self.config.throttle)
    }
}

fn map_brake(value: f32, abs_active: bool, config: &BrakeEffectConfig) -> TriggerEffect {
    if !config.enabled {
        return TriggerEffect::Normal;
    }
    if abs_active {
        return TriggerEffect::Pulse {
            start: config.start_position.min(10),
            force: config.abs_pulse_force.min(10),
            frequency: config.abs_pulse_frequency.min(10),
        };
    }
    resistance(
        value,
        config.deadzone,
        config.min_force,
        config.max_force,
        config.start_position,
    )
}

fn map_throttle(value: f32, tc_active: bool, config: &ThrottleEffectConfig) -> TriggerEffect {
    if !config.enabled {
        return TriggerEffect::Normal;
    }
    if tc_active {
        return TriggerEffect::Pulse {
            start: config.start_position.min(10),
            force: config.tc_pulse_force.min(10),
            frequency: config.tc_pulse_frequency.min(10),
        };
    }
    resistance(
        value,
        config.deadzone,
        config.min_force,
        config.max_force,
        config.start_position,
    )
}

fn resistance(value: f32, deadzone: f32, min: u8, max: u8, start: u8) -> TriggerEffect {
    let value = value.clamp(0.0, 1.0);
    if value < deadzone {
        return TriggerEffect::Normal;
    }

    let normalized = if deadzone < 1.0 {
        (value - deadzone) / (1.0 - deadzone)
    } else {
        1.0
    };
    let force = f32::from(min) + normalized * f32::from(max.saturating_sub(min));

    TriggerEffect::Resistance {
        start: start.min(10),
        force: force.round().clamp(0.0, 10.0) as u8,
    }
}

#[cfg(test)]
mod tests {
    use crate::config::{EffectConfig, SmoothingConfig};

    use super::*;

    fn mapper() -> EffectMapper {
        EffectMapper::new(
            EffectConfig::default(),
            SmoothingConfig {
                enabled: false,
                ..SmoothingConfig::default()
            },
        )
    }

    fn connected_frame() -> TelemetryFrame {
        TelemetryFrame {
            connected: true,
            player_has_vehicle: true,
            max_rpm: 8_000.0,
            ..TelemetryFrame::default()
        }
    }

    #[test]
    fn maps_brake_to_force() {
        let mut frame = connected_frame();
        frame.brake = 1.0;
        let output = mapper().map(frame);
        assert_eq!(
            output.left,
            TriggerEffect::Resistance { start: 2, force: 8 }
        );
    }

    #[test]
    fn maps_throttle_to_force() {
        let mut frame = connected_frame();
        frame.throttle = 1.0;
        let output = mapper().map(frame);
        assert_eq!(
            output.right,
            TriggerEffect::Resistance { start: 2, force: 4 }
        );
    }

    #[test]
    fn deadzone_maps_to_normal() {
        let mut frame = connected_frame();
        frame.brake = 0.01;
        frame.throttle = 0.01;
        let output = mapper().map(frame);
        assert_eq!(output.left, TriggerEffect::Normal);
        assert_eq!(output.right, TriggerEffect::Normal);
    }

    #[test]
    fn abs_overrides_brake() {
        let mut frame = connected_frame();
        frame.brake = 0.9;
        frame.abs_active = true;
        assert!(matches!(
            mapper().map(frame).left,
            TriggerEffect::Pulse { .. }
        ));
    }

    #[test]
    fn tc_overrides_throttle() {
        let mut frame = connected_frame();
        frame.throttle = 0.9;
        frame.tc_active = true;
        assert!(matches!(
            mapper().map(frame).right,
            TriggerEffect::Pulse { .. }
        ));
    }

    #[test]
    fn rev_limiter_overrides_tc() {
        let mut frame = connected_frame();
        frame.throttle = 1.0;
        frame.tc_active = true;
        frame.rpm = 7_900.0;
        assert!(matches!(
            mapper().map(frame).right,
            TriggerEffect::Vibrate { .. }
        ));
    }

    #[test]
    fn disconnect_resets_smoothing_state() {
        let mut mapper = EffectMapper::new(
            EffectConfig::default(),
            SmoothingConfig {
                enabled: true,
                attack: 0.5,
                release: 0.25,
            },
        );
        let mut frame = connected_frame();
        frame.brake = 1.0;
        mapper.map(frame);
        mapper.map(TelemetryFrame::default());

        let mut reconnected = connected_frame();
        reconnected.brake = 0.04;
        assert_eq!(mapper.map(reconnected).left, TriggerEffect::Normal);
    }
}
