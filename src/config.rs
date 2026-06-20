use std::{fmt, fs, path::PathBuf};

use anyhow::{bail, Context, Result};
use clap::{Parser, ValueEnum};
use serde::{Deserialize, Serialize};

#[derive(Debug, Parser)]
#[command(version, about = "LMU telemetry to DualSense adaptive-trigger bridge")]
pub struct Cli {
    #[arg(long)]
    pub config: Option<PathBuf>,

    #[arg(long, value_enum)]
    pub telemetry: Option<TelemetrySource>,

    #[arg(long, value_enum)]
    pub output: Option<OutputKind>,

    #[arg(long)]
    pub tick_hz: Option<u32>,

    #[arg(long)]
    pub dsx_host: Option<String>,

    #[arg(long)]
    pub dsx_port: Option<u16>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum TelemetrySource {
    Mock,
    Lmu,
}

impl fmt::Display for TelemetrySource {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Mock => "mock",
            Self::Lmu => "lmu",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum OutputKind {
    Null,
    DsxUdp,
}

impl fmt::Display for OutputKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Null => "null",
            Self::DsxUdp => "dsx_udp",
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct AppConfig {
    pub app: AppSection,
    pub lmu: LmuConfig,
    pub dsx: DsxConfig,
    pub effects: EffectConfig,
    pub smoothing: SmoothingConfig,
}

impl AppConfig {
    pub fn load(cli: &Cli) -> Result<Self> {
        let mut config = if let Some(path) = &cli.config {
            let contents = fs::read_to_string(path)
                .with_context(|| format!("failed to read config file {}", path.display()))?;
            toml::from_str(&contents)
                .with_context(|| format!("failed to parse config file {}", path.display()))?
        } else {
            Self::default()
        };

        if let Some(source) = cli.telemetry {
            config.app.telemetry_source = source;
        }
        if let Some(output) = cli.output {
            config.app.output = output;
        }
        if let Some(tick_hz) = cli.tick_hz {
            config.app.tick_hz = tick_hz;
        }
        if let Some(host) = &cli.dsx_host {
            config.dsx.host.clone_from(host);
        }
        if let Some(port) = cli.dsx_port {
            config.dsx.port = Some(port);
        }

        config.validate()?;
        Ok(config)
    }

    fn validate(&self) -> Result<()> {
        if self.app.tick_hz == 0 {
            bail!("app.tick_hz must be greater than zero");
        }
        if self.dsx.host.trim().is_empty() {
            bail!("dsx.host must not be empty");
        }
        if !matches!(
            self.app.log_level.as_str(),
            "trace" | "debug" | "info" | "warn" | "error"
        ) {
            bail!("app.log_level must be trace, debug, info, warn, or error");
        }

        validate_force("effects.brake.min_force", self.effects.brake.min_force)?;
        validate_force("effects.brake.max_force", self.effects.brake.max_force)?;
        validate_force(
            "effects.brake.abs_pulse_force",
            self.effects.brake.abs_pulse_force,
        )?;
        validate_force(
            "effects.throttle.min_force",
            self.effects.throttle.min_force,
        )?;
        validate_force(
            "effects.throttle.max_force",
            self.effects.throttle.max_force,
        )?;
        validate_force(
            "effects.throttle.tc_pulse_force",
            self.effects.throttle.tc_pulse_force,
        )?;
        validate_force(
            "effects.rpm.vibration_force",
            self.effects.rpm.vibration_force,
        )?;

        if self.effects.brake.min_force > self.effects.brake.max_force {
            bail!("effects.brake.min_force must not exceed max_force");
        }
        if self.effects.throttle.min_force > self.effects.throttle.max_force {
            bail!("effects.throttle.min_force must not exceed max_force");
        }
        for (name, value) in [
            ("effects.brake.deadzone", self.effects.brake.deadzone),
            ("effects.throttle.deadzone", self.effects.throttle.deadzone),
            (
                "effects.rpm.rev_limit_ratio",
                self.effects.rpm.rev_limit_ratio,
            ),
            ("smoothing.attack", self.smoothing.attack),
            ("smoothing.release", self.smoothing.release),
        ] {
            if !(0.0..=1.0).contains(&value) {
                bail!("{name} must be within 0.0..=1.0");
            }
        }

        Ok(())
    }
}

fn validate_force(name: &str, value: u8) -> Result<()> {
    if value > 10 {
        bail!("{name} must be within 0..=10");
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppSection {
    pub tick_hz: u32,
    pub telemetry_source: TelemetrySource,
    pub output: OutputKind,
    pub log_level: String,
}

impl Default for AppSection {
    fn default() -> Self {
        Self {
            tick_hz: 60,
            telemetry_source: TelemetrySource::Mock,
            output: OutputKind::Null,
            log_level: "info".to_owned(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LmuConfig {
    pub shared_memory_name: String,
    pub header_path: String,
}

impl Default for LmuConfig {
    fn default() -> Self {
        Self {
            shared_memory_name: "LMU_Data".to_owned(),
            header_path: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DsxConfig {
    pub host: String,
    pub port: Option<u16>,
    pub controller_index: u8,
}

impl Default for DsxConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_owned(),
            port: None,
            controller_index: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct EffectConfig {
    pub brake: BrakeEffectConfig,
    pub throttle: ThrottleEffectConfig,
    pub rpm: RpmEffectConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct BrakeEffectConfig {
    pub enabled: bool,
    pub deadzone: f32,
    pub min_force: u8,
    pub max_force: u8,
    pub start_position: u8,
    pub abs_pulse_force: u8,
    pub abs_pulse_frequency: u8,
}

impl Default for BrakeEffectConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            deadzone: 0.03,
            min_force: 1,
            max_force: 8,
            start_position: 2,
            abs_pulse_force: 9,
            abs_pulse_frequency: 8,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ThrottleEffectConfig {
    pub enabled: bool,
    pub deadzone: f32,
    pub min_force: u8,
    pub max_force: u8,
    pub start_position: u8,
    pub tc_pulse_force: u8,
    pub tc_pulse_frequency: u8,
}

impl Default for ThrottleEffectConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            deadzone: 0.03,
            min_force: 0,
            max_force: 4,
            start_position: 2,
            tc_pulse_force: 6,
            tc_pulse_frequency: 7,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct RpmEffectConfig {
    pub enabled: bool,
    pub rev_limit_ratio: f32,
    pub vibration_force: u8,
    pub vibration_frequency: u8,
}

impl Default for RpmEffectConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            rev_limit_ratio: 0.97,
            vibration_force: 7,
            vibration_frequency: 10,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SmoothingConfig {
    pub enabled: bool,
    pub attack: f32,
    pub release: f32,
}

impl Default for SmoothingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            attack: 0.45,
            release: 0.25,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cli() -> Cli {
        Cli {
            config: None,
            telemetry: None,
            output: None,
            tick_hz: None,
            dsx_host: None,
            dsx_port: None,
        }
    }

    #[test]
    fn defaults_are_mock_and_null() {
        let config = AppConfig::load(&cli()).unwrap();
        assert_eq!(config.app.telemetry_source, TelemetrySource::Mock);
        assert_eq!(config.app.output, OutputKind::Null);
        assert_eq!(config.app.tick_hz, 60);
    }

    #[test]
    fn parses_partial_toml_with_defaults() {
        let config: AppConfig = toml::from_str(
            r#"
                [app]
                tick_hz = 120

                [effects.brake]
                max_force = 6
            "#,
        )
        .unwrap();

        assert_eq!(config.app.tick_hz, 120);
        assert_eq!(config.effects.brake.max_force, 6);
        assert!(config.effects.throttle.enabled);
    }

    #[test]
    fn cli_overrides_config_values() {
        let mut cli = cli();
        cli.telemetry = Some(TelemetrySource::Lmu);
        cli.output = Some(OutputKind::DsxUdp);
        cli.tick_hz = Some(30);
        cli.dsx_port = Some(7777);

        let config = AppConfig::load(&cli).unwrap();
        assert_eq!(config.app.telemetry_source, TelemetrySource::Lmu);
        assert_eq!(config.app.output, OutputKind::DsxUdp);
        assert_eq!(config.app.tick_hz, 30);
        assert_eq!(config.dsx.port, Some(7777));
    }

    #[test]
    fn rejects_invalid_tick_rate() {
        let mut cli = cli();
        cli.tick_hz = Some(0);
        assert!(AppConfig::load(&cli).is_err());
    }
}
