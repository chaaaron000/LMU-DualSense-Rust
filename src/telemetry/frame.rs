#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TelemetryFrame {
    pub connected: bool,
    pub player_has_vehicle: bool,
    pub throttle: f32,
    pub brake: f32,
    pub clutch: f32,
    pub steering: f32,
    pub rpm: f32,
    pub max_rpm: f32,
    pub gear: i32,
    pub abs_active: bool,
    pub tc_active: bool,
    pub abs_level: i32,
    pub tc_level: i32,
    pub tc_slip_level: i32,
    pub tc_cut_level: i32,
    pub max_wheel_lock_ratio: f32,
    pub max_wheel_slip_ratio: f32,
    pub speed_mps: f32,
    pub pit_limiter_active: bool,
}

impl Default for TelemetryFrame {
    fn default() -> Self {
        Self {
            connected: false,
            player_has_vehicle: false,
            throttle: 0.0,
            brake: 0.0,
            clutch: 0.0,
            steering: 0.0,
            rpm: 0.0,
            max_rpm: 0.0,
            gear: 0,
            abs_active: false,
            tc_active: false,
            abs_level: 0,
            tc_level: 0,
            tc_slip_level: 0,
            tc_cut_level: 0,
            max_wheel_lock_ratio: 0.0,
            max_wheel_slip_ratio: 0.0,
            speed_mps: 0.0,
            pit_limiter_active: false,
        }
    }
}
