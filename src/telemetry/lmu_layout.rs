use anyhow::{bail, ensure, Context, Result};

use super::TelemetryFrame;

pub const SNAPSHOT_SIZE: usize = 324_820;
pub const MAX_VEHICLES: usize = 104;

const GAME_VERSION_OFFSET: usize = 64;
const TELEMETRY_UPDATE_EVENT_OFFSET: usize = 44;
const TELEMETRY_OFFSET: usize = 128_464;
const ACTIVE_VEHICLES_OFFSET: usize = TELEMETRY_OFFSET;
const PLAYER_VEHICLE_INDEX_OFFSET: usize = TELEMETRY_OFFSET + 1;
const PLAYER_HAS_VEHICLE_OFFSET: usize = TELEMETRY_OFFSET + 2;
const VEHICLES_OFFSET: usize = TELEMETRY_OFFSET + 4;
const VEHICLE_SIZE: usize = 1_888;

const LOCAL_VELOCITY_OFFSET: usize = 184;
const LOCAL_ACCELERATION_OFFSET: usize = 208;
const GEAR_OFFSET: usize = 352;
const ENGINE_RPM_OFFSET: usize = 356;
const THROTTLE_OFFSET: usize = 388;
const BRAKE_OFFSET: usize = 396;
const STEERING_OFFSET: usize = 404;
const CLUTCH_OFFSET: usize = 412;
const ENGINE_MAX_RPM_OFFSET: usize = 532;
const ABS_ACTIVE_OFFSET: usize = 746;
const TC_ACTIVE_OFFSET: usize = 747;
const SPEED_LIMITER_ACTIVE_OFFSET: usize = 748;
const TC_LEVEL_OFFSET: usize = 750;
const TC_SLIP_LEVEL_OFFSET: usize = 752;
const TC_CUT_LEVEL_OFFSET: usize = 754;
const ABS_LEVEL_OFFSET: usize = 756;
const WHEELS_OFFSET: usize = 848;
const WHEEL_SIZE: usize = 260;
const WHEEL_ROTATION_OFFSET: usize = 40;
const MIN_SLIP_SPEED_MPS: f64 = 1.0;
const MIN_AXLE_ROTATION: f64 = 4.0;
const MAX_ROTATION_BIAS: f64 = 0.002;

#[derive(Default)]
pub struct WheelLockEstimator {
    player_index: Option<usize>,
    radius_front_ema: f64,
    radius_rear_ema: f64,
    last_acceleration: f64,
}

impl WheelLockEstimator {
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    fn select_player(&mut self, player_index: usize) {
        if self.player_index != Some(player_index) {
            self.reset();
            self.player_index = Some(player_index);
        }
    }

    fn update(&mut self, wheel_rotation: [f64; 4], speed: f64, acceleration: f64) -> (f32, f32) {
        let axle_front = wheel_axle_rotation(wheel_rotation[0], wheel_rotation[1]);
        let axle_rear = wheel_axle_rotation(wheel_rotation[2], wheel_rotation[3]);
        let bias_front = wheel_rotation_bias(axle_front, wheel_rotation[0], wheel_rotation[1]);
        let bias_rear = wheel_rotation_bias(axle_rear, wheel_rotation[2], wheel_rotation[3]);

        if self.last_acceleration != acceleration {
            self.last_acceleration = acceleration;
            let factor = 2.0 / (40.0 * acceleration).max(20.0);

            if axle_front < -MIN_AXLE_ROTATION && bias_front > 0.0 && bias_front < MAX_ROTATION_BIAS
            {
                let radius = speed / axle_front.abs();
                self.radius_front_ema += factor * (radius - self.radius_front_ema);
            }
            if axle_rear < -MIN_AXLE_ROTATION && bias_rear > 0.0 && bias_rear < MAX_ROTATION_BIAS {
                let radius = speed / axle_rear.abs();
                self.radius_rear_ema += factor * (radius - self.radius_rear_ema);
            }
        }

        let slip_ratio = [
            wheel_slip_ratio(wheel_rotation[0], self.radius_front_ema, speed),
            wheel_slip_ratio(wheel_rotation[1], self.radius_front_ema, speed),
            wheel_slip_ratio(wheel_rotation[2], self.radius_rear_ema, speed),
            wheel_slip_ratio(wheel_rotation[3], self.radius_rear_ema, speed),
        ];
        let lock_ratio = slip_ratio
            .into_iter()
            .fold(f64::INFINITY, f64::min)
            .abs()
            .min(1.0) as f32;
        let wheel_slip_ratio = slip_ratio
            .into_iter()
            .fold(f64::NEG_INFINITY, f64::max)
            .clamp(0.0, 1.0) as f32;
        (lock_ratio, wheel_slip_ratio)
    }
}

pub fn parse_snapshot(bytes: &[u8], wheel_lock: &mut WheelLockEstimator) -> Result<TelemetryFrame> {
    ensure!(
        bytes.len() >= SNAPSHOT_SIZE,
        "LMU snapshot is too small: expected {SNAPSHOT_SIZE}, got {}",
        bytes.len()
    );

    let game_version = read_i32(bytes, GAME_VERSION_OFFSET)?;
    if game_version <= 0 {
        wheel_lock.reset();
        return Ok(TelemetryFrame::default());
    }

    let telemetry_updated = read_u32(bytes, TELEMETRY_UPDATE_EVENT_OFFSET)? != 0;
    let active_vehicles = usize::from(read_u8(bytes, ACTIVE_VEHICLES_OFFSET)?);
    ensure!(
        active_vehicles <= MAX_VEHICLES,
        "LMU active vehicle count {active_vehicles} exceeds {MAX_VEHICLES}"
    );

    let player_has_vehicle = read_bool(bytes, PLAYER_HAS_VEHICLE_OFFSET)?;
    if !telemetry_updated || !player_has_vehicle {
        wheel_lock.reset();
        return Ok(TelemetryFrame {
            connected: true,
            player_has_vehicle: false,
            ..TelemetryFrame::default()
        });
    }

    let player_index = usize::from(read_u8(bytes, PLAYER_VEHICLE_INDEX_OFFSET)?);
    ensure!(
        player_index < active_vehicles && player_index < MAX_VEHICLES,
        "LMU player vehicle index {player_index} is invalid for {active_vehicles} active vehicles"
    );
    wheel_lock.select_player(player_index);

    let vehicle_offset = VEHICLES_OFFSET
        .checked_add(player_index * VEHICLE_SIZE)
        .context("LMU player vehicle offset overflowed")?;

    let throttle = read_unit_f32(bytes, vehicle_offset + THROTTLE_OFFSET, "throttle")?;
    let brake = read_unit_f32(bytes, vehicle_offset + BRAKE_OFFSET, "brake")?;
    let clutch = read_unit_f32(bytes, vehicle_offset + CLUTCH_OFFSET, "clutch")?;
    let steering = read_f32_range(
        bytes,
        vehicle_offset + STEERING_OFFSET,
        -1.0..=1.0,
        "steering",
    )?;
    let rpm = read_non_negative_f32(bytes, vehicle_offset + ENGINE_RPM_OFFSET, "rpm")?;
    let max_rpm = read_non_negative_f32(bytes, vehicle_offset + ENGINE_MAX_RPM_OFFSET, "max_rpm")?;
    ensure!(max_rpm > 0.0, "LMU max_rpm must be greater than zero");

    let gear = read_i32(bytes, vehicle_offset + GEAR_OFFSET)?;
    ensure!((-1..=20).contains(&gear), "LMU gear {gear} is out of range");

    let velocity_x = read_finite_f64(bytes, vehicle_offset + LOCAL_VELOCITY_OFFSET, "velocity_x")?;
    let velocity_y = read_finite_f64(
        bytes,
        vehicle_offset + LOCAL_VELOCITY_OFFSET + 8,
        "velocity_y",
    )?;
    let velocity_z = read_finite_f64(
        bytes,
        vehicle_offset + LOCAL_VELOCITY_OFFSET + 16,
        "velocity_z",
    )?;
    let speed_mps = velocity_x
        .hypot(velocity_y)
        .hypot(velocity_z)
        .clamp(0.0, f64::from(f32::MAX)) as f32;
    let acceleration_x = read_finite_f64(
        bytes,
        vehicle_offset + LOCAL_ACCELERATION_OFFSET,
        "acceleration_x",
    )?;
    let acceleration_z = read_finite_f64(
        bytes,
        vehicle_offset + LOCAL_ACCELERATION_OFFSET + 16,
        "acceleration_z",
    )?;
    let wheel_rotation = read_wheel_rotation(bytes, vehicle_offset)?;
    let (max_wheel_lock_ratio, max_wheel_slip_ratio) = wheel_lock.update(
        wheel_rotation,
        f64::from(speed_mps),
        acceleration_x.abs().max(acceleration_z.abs()),
    );

    Ok(TelemetryFrame {
        connected: true,
        player_has_vehicle: true,
        throttle,
        brake,
        clutch,
        steering,
        rpm,
        max_rpm,
        gear,
        abs_active: read_bool(bytes, vehicle_offset + ABS_ACTIVE_OFFSET)?,
        tc_active: read_bool(bytes, vehicle_offset + TC_ACTIVE_OFFSET)?,
        abs_level: i32::from(read_u8(bytes, vehicle_offset + ABS_LEVEL_OFFSET)?),
        tc_level: i32::from(read_u8(bytes, vehicle_offset + TC_LEVEL_OFFSET)?),
        tc_slip_level: i32::from(read_u8(bytes, vehicle_offset + TC_SLIP_LEVEL_OFFSET)?),
        tc_cut_level: i32::from(read_u8(bytes, vehicle_offset + TC_CUT_LEVEL_OFFSET)?),
        max_wheel_lock_ratio,
        max_wheel_slip_ratio,
        speed_mps,
        pit_limiter_active: read_bool(bytes, vehicle_offset + SPEED_LIMITER_ACTIVE_OFFSET)?,
    })
}

fn read_wheel_rotation(bytes: &[u8], vehicle_offset: usize) -> Result<[f64; 4]> {
    let mut rotation = [0.0; 4];
    for (wheel_index, value) in rotation.iter_mut().enumerate() {
        let wheel = vehicle_offset + WHEELS_OFFSET + wheel_index * WHEEL_SIZE;
        *value = read_finite_f64(bytes, wheel + WHEEL_ROTATION_OFFSET, "wheel rotation")?;
    }
    Ok(rotation)
}

fn wheel_axle_rotation(left: f64, right: f64) -> f64 {
    if (left >= 0.0 && right >= 0.0) || (left <= 0.0 && right <= 0.0) {
        (left + right) / 2.0
    } else {
        0.0
    }
}

fn wheel_rotation_bias(axle: f64, left: f64, right: f64) -> f64 {
    if axle != 0.0 {
        ((left - right) / axle).abs()
    } else {
        0.0
    }
}

fn wheel_slip_ratio(rotation: f64, radius: f64, speed: f64) -> f64 {
    if speed > MIN_SLIP_SPEED_MPS {
        rotation.abs() * radius / speed - 1.0
    } else {
        0.0
    }
}

fn read_u8(bytes: &[u8], offset: usize) -> Result<u8> {
    bytes
        .get(offset)
        .copied()
        .with_context(|| format!("LMU byte offset {offset} is outside the snapshot"))
}

fn read_bool(bytes: &[u8], offset: usize) -> Result<bool> {
    match read_u8(bytes, offset)? {
        0 => Ok(false),
        1 => Ok(true),
        value => bail!("LMU bool at offset {offset} has invalid value {value}"),
    }
}

fn read_i32(bytes: &[u8], offset: usize) -> Result<i32> {
    Ok(i32::from_le_bytes(read_array(bytes, offset)?))
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32> {
    Ok(u32::from_le_bytes(read_array(bytes, offset)?))
}

fn read_finite_f64(bytes: &[u8], offset: usize, name: &str) -> Result<f64> {
    let value = f64::from_le_bytes(read_array(bytes, offset)?);
    ensure!(value.is_finite(), "LMU {name} is not finite");
    Ok(value)
}

fn read_non_negative_f32(bytes: &[u8], offset: usize, name: &str) -> Result<f32> {
    read_f32_range(bytes, offset, 0.0..=f32::MAX, name)
}

fn read_unit_f32(bytes: &[u8], offset: usize, name: &str) -> Result<f32> {
    read_f32_range(bytes, offset, 0.0..=1.0, name)
}

fn read_f32_range(
    bytes: &[u8],
    offset: usize,
    range: std::ops::RangeInclusive<f32>,
    name: &str,
) -> Result<f32> {
    let value = read_finite_f64(bytes, offset, name)?;
    ensure!(
        value >= f64::from(*range.start()) && value <= f64::from(*range.end()),
        "LMU {name} value {value} is outside {:?}",
        range
    );
    Ok(value as f32)
}

fn read_array<const N: usize>(bytes: &[u8], offset: usize) -> Result<[u8; N]> {
    let end = offset.checked_add(N).context("LMU byte range overflowed")?;
    bytes
        .get(offset..end)
        .with_context(|| format!("LMU byte range {offset}..{end} is outside the snapshot"))?
        .try_into()
        .context("LMU byte range has an unexpected length")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_u8(bytes: &mut [u8], offset: usize, value: u8) {
        bytes[offset] = value;
    }

    fn write_i32(bytes: &mut [u8], offset: usize, value: i32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn write_u32(bytes: &mut [u8], offset: usize, value: u32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn write_f64(bytes: &mut [u8], offset: usize, value: f64) {
        bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
    }

    fn parse_once(bytes: &[u8]) -> Result<TelemetryFrame> {
        parse_snapshot(bytes, &mut WheelLockEstimator::default())
    }

    fn valid_snapshot(player_index: usize) -> Vec<u8> {
        let mut bytes = vec![0; SNAPSHOT_SIZE];
        write_i32(&mut bytes, GAME_VERSION_OFFSET, 1);
        write_u32(&mut bytes, TELEMETRY_UPDATE_EVENT_OFFSET, 1);
        write_u8(&mut bytes, ACTIVE_VEHICLES_OFFSET, 104);
        write_u8(&mut bytes, PLAYER_VEHICLE_INDEX_OFFSET, player_index as u8);
        write_u8(&mut bytes, PLAYER_HAS_VEHICLE_OFFSET, 1);

        let vehicle = VEHICLES_OFFSET + player_index * VEHICLE_SIZE;
        write_f64(&mut bytes, vehicle + LOCAL_VELOCITY_OFFSET, 3.0);
        write_f64(&mut bytes, vehicle + LOCAL_VELOCITY_OFFSET + 8, 4.0);
        write_f64(&mut bytes, vehicle + LOCAL_VELOCITY_OFFSET + 16, 12.0);
        write_f64(&mut bytes, vehicle + LOCAL_ACCELERATION_OFFSET, 0.5);
        write_f64(&mut bytes, vehicle + LOCAL_ACCELERATION_OFFSET + 16, 0.25);
        write_i32(&mut bytes, vehicle + GEAR_OFFSET, 4);
        write_f64(&mut bytes, vehicle + ENGINE_RPM_OFFSET, 7_500.0);
        write_f64(&mut bytes, vehicle + THROTTLE_OFFSET, 0.75);
        write_f64(&mut bytes, vehicle + BRAKE_OFFSET, 0.5);
        write_f64(&mut bytes, vehicle + STEERING_OFFSET, -0.25);
        write_f64(&mut bytes, vehicle + CLUTCH_OFFSET, 0.1);
        write_f64(&mut bytes, vehicle + ENGINE_MAX_RPM_OFFSET, 8_000.0);
        write_u8(&mut bytes, vehicle + ABS_ACTIVE_OFFSET, 1);
        write_u8(&mut bytes, vehicle + TC_ACTIVE_OFFSET, 1);
        write_u8(&mut bytes, vehicle + SPEED_LIMITER_ACTIVE_OFFSET, 1);
        write_u8(&mut bytes, vehicle + TC_LEVEL_OFFSET, 5);
        write_u8(&mut bytes, vehicle + TC_SLIP_LEVEL_OFFSET, 3);
        write_u8(&mut bytes, vehicle + TC_CUT_LEVEL_OFFSET, 2);
        write_u8(&mut bytes, vehicle + ABS_LEVEL_OFFSET, 4);
        for wheel_index in 0..4 {
            let wheel = vehicle + WHEELS_OFFSET + wheel_index * WHEEL_SIZE;
            let rotation = if wheel_index % 2 == 0 { -40.0 } else { -40.04 };
            write_f64(&mut bytes, wheel + WHEEL_ROTATION_OFFSET, rotation);
        }
        bytes
    }

    #[test]
    fn parses_player_telemetry_fields() {
        let frame = parse_once(&valid_snapshot(3)).unwrap();
        assert!(frame.connected);
        assert!(frame.player_has_vehicle);
        assert_eq!(frame.throttle, 0.75);
        assert_eq!(frame.brake, 0.5);
        assert_eq!(frame.clutch, 0.1);
        assert_eq!(frame.steering, -0.25);
        assert_eq!(frame.rpm, 7_500.0);
        assert_eq!(frame.max_rpm, 8_000.0);
        assert_eq!(frame.gear, 4);
        assert!(frame.abs_active);
        assert!(frame.tc_active);
        assert_eq!(frame.abs_level, 4);
        assert_eq!(frame.tc_level, 5);
        assert_eq!(frame.tc_slip_level, 3);
        assert_eq!(frame.tc_cut_level, 2);
        assert_eq!(frame.speed_mps, 13.0);
        assert!(frame.pit_limiter_active);
    }

    #[test]
    fn accepts_last_vehicle_slot() {
        assert!(parse_once(&valid_snapshot(103)).is_ok());
    }

    #[test]
    fn rejects_invalid_player_index() {
        let mut bytes = valid_snapshot(3);
        write_u8(&mut bytes, ACTIVE_VEHICLES_OFFSET, 3);
        assert!(parse_once(&bytes).is_err());
    }

    #[test]
    fn rejects_too_many_active_vehicles() {
        let mut bytes = valid_snapshot(0);
        write_u8(&mut bytes, ACTIVE_VEHICLES_OFFSET, 105);
        assert!(parse_once(&bytes).is_err());
    }

    #[test]
    fn rejects_truncated_snapshot() {
        assert!(parse_once(&vec![0; SNAPSHOT_SIZE - 1]).is_err());
    }

    #[test]
    fn rejects_invalid_bool() {
        let mut bytes = valid_snapshot(0);
        write_u8(&mut bytes, PLAYER_HAS_VEHICLE_OFFSET, 2);
        assert!(parse_once(&bytes).is_err());
    }

    #[test]
    fn rejects_non_finite_and_out_of_range_values() {
        let mut bytes = valid_snapshot(0);
        write_f64(&mut bytes, VEHICLES_OFFSET + THROTTLE_OFFSET, f64::NAN);
        assert!(parse_once(&bytes).is_err());

        let mut bytes = valid_snapshot(0);
        write_f64(&mut bytes, VEHICLES_OFFSET + BRAKE_OFFSET, 1.1);
        assert!(parse_once(&bytes).is_err());
    }

    #[test]
    fn handles_disconnected_and_no_vehicle_states() {
        let bytes = vec![0; SNAPSHOT_SIZE];
        assert_eq!(parse_once(&bytes).unwrap(), TelemetryFrame::default());

        let mut bytes = valid_snapshot(0);
        write_u8(&mut bytes, PLAYER_HAS_VEHICLE_OFFSET, 0);
        let frame = parse_once(&bytes).unwrap();
        assert!(frame.connected);
        assert!(!frame.player_has_vehicle);
    }

    #[test]
    fn estimates_locked_wheel_ratio_with_calibrated_radius() {
        let mut estimator = WheelLockEstimator::default();
        for sample in 0..100 {
            estimator.update(
                [-40.0, -40.04, -40.0, -40.04],
                13.0,
                0.5 + f64::from(sample) * 0.001,
            );
        }

        let (lock_ratio, _) = estimator.update([-20.0, -40.04, -40.0, -40.04], 13.0, 0.7);
        assert!((lock_ratio - 0.5).abs() < 0.01);
    }

    #[test]
    fn wheel_lock_estimator_returns_zero_below_one_meter_per_second() {
        let (lock_ratio, wheel_slip_ratio) =
            WheelLockEstimator::default().update([-1.0, -1.001, -1.0, -1.001], 1.0, 0.5);
        assert_eq!(lock_ratio, 0.0);
        assert_eq!(wheel_slip_ratio, 0.0);
    }

    #[test]
    fn estimates_driven_wheel_slip_with_calibrated_radius() {
        let mut estimator = WheelLockEstimator::default();
        for sample in 0..100 {
            estimator.update(
                [-40.0, -40.04, -40.0, -40.04],
                13.0,
                0.5 + f64::from(sample) * 0.001,
            );
        }

        let (_, wheel_slip_ratio) = estimator.update([-40.0, -40.04, -48.0, -48.04], 13.0, 0.7);
        assert!((0.18..0.20).contains(&wheel_slip_ratio));
    }
}
