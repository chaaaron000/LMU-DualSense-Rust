use anyhow::Result;
use clap::Parser;
use lmu_dualsense_bridge::{
    app::App,
    config::{AppConfig, Cli, OutputKind, TelemetrySource},
    effects::EffectMapper,
    output::{DsxUdpOutput, NullOutput, TriggerOutput},
    telemetry::{LmuSharedMemoryReader, MockTelemetryReader, TelemetryReader},
};
use tracing::info;
use tracing_subscriber::filter::LevelFilter;

fn main() -> Result<()> {
    let cli = Cli::parse();
    let config = AppConfig::load(&cli)?;
    init_logging(&config.app.log_level);

    info!(
        "[APP] Started | telemetry={} | output={} | tick={} Hz",
        config.app.telemetry_source, config.app.output, config.app.tick_hz
    );

    let reader: Box<dyn TelemetryReader> = match config.app.telemetry_source {
        TelemetrySource::Mock => Box::new(MockTelemetryReader::new(config.app.tick_hz)),
        TelemetrySource::Lmu => Box::new(LmuSharedMemoryReader::new(config.lmu.clone())),
    };

    let output: Box<dyn TriggerOutput> = match config.app.output {
        OutputKind::Null => Box::new(NullOutput),
        OutputKind::DsxUdp => Box::new(DsxUdpOutput::new(&config.dsx)?),
    };

    let mapper = EffectMapper::new(config.effects.clone(), config.smoothing.clone());
    App::new(reader, mapper, output, config.app.tick_hz).run()
}

fn init_logging(level: &str) {
    let level = match level {
        "trace" => LevelFilter::TRACE,
        "debug" => LevelFilter::DEBUG,
        "warn" => LevelFilter::WARN,
        "error" => LevelFilter::ERROR,
        _ => LevelFilter::INFO,
    };

    tracing_subscriber::fmt()
        .with_max_level(level)
        .without_time()
        .with_target(false)
        .compact()
        .init();
}
