use std::{
    fs,
    net::{SocketAddr, ToSocketAddrs, UdpSocket},
    path::Path,
};

use anyhow::{Context, Result};
use serde::Serialize;
use serde_json::{json, Value};
use tracing::info;

use crate::{
    config::DsxConfig,
    effects::{TriggerEffect, TriggerOutputFrame},
};

use super::TriggerOutput;

const DSX_PORT_FILE: &str = r"C:\Temp\DualSenseX\DualSenseX_PortNumber.txt";
const FALLBACK_PORT: u16 = 6969;

pub struct DsxUdpOutput {
    socket: UdpSocket,
    target: SocketAddr,
    encoder: DsxPacketEncoder,
}

impl DsxUdpOutput {
    pub fn new(config: &DsxConfig) -> Result<Self> {
        let port = resolve_port(config.port);
        let target = format!("{}:{port}", config.host)
            .to_socket_addrs()
            .with_context(|| format!("failed to resolve DSX host {}", config.host))?
            .next()
            .context("DSX host resolved to no socket addresses")?;
        let socket = UdpSocket::bind("0.0.0.0:0").context("failed to bind UDP socket")?;

        info!(%target, "DSX UDP output ready");
        Ok(Self {
            socket,
            target,
            encoder: DsxPacketEncoder::new(config.controller_index),
        })
    }
}

impl TriggerOutput for DsxUdpOutput {
    fn send(&mut self, frame: &TriggerOutputFrame) -> Result<()> {
        for packet in self.encoder.encode_frame(frame)? {
            self.socket
                .send_to(packet.as_bytes(), self.target)
                .with_context(|| format!("failed to send DSX UDP packet to {}", self.target))?;
        }
        Ok(())
    }
}

fn resolve_port(configured: Option<u16>) -> u16 {
    configured
        .or_else(|| read_port_file(Path::new(DSX_PORT_FILE)))
        .unwrap_or(FALLBACK_PORT)
}

fn read_port_file(path: &Path) -> Option<u16> {
    fs::read_to_string(path).ok()?.trim().parse().ok()
}

#[derive(Debug, Clone)]
pub struct DsxPacketEncoder {
    controller_index: u8,
}

impl DsxPacketEncoder {
    pub fn new(controller_index: u8) -> Self {
        Self { controller_index }
    }

    pub fn encode_frame(&self, frame: &TriggerOutputFrame) -> Result<[String; 2]> {
        Ok([
            self.encode_trigger(1, &frame.left)?,
            self.encode_trigger(2, &frame.right)?,
        ])
    }

    fn encode_trigger(&self, trigger: u8, effect: &TriggerEffect) -> Result<String> {
        let mut parameters = vec![json!(self.controller_index), json!(trigger)];
        match effect {
            TriggerEffect::Normal => parameters.push(json!(0)),
            TriggerEffect::Resistance { start, force } => {
                parameters.push(json!(13));
                parameters.push(json!((*start).min(9)));
                parameters.push(json!(scale(*force, 8)));
            }
            TriggerEffect::Pulse { .. } => {
                // Steam DSX v2's VibrateTriggerPulse preset does not expose the
                // internal start/force/frequency values.
                parameters.push(json!(11));
            }
            TriggerEffect::Vibrate { force, .. } => {
                // Steam DSX v2 VibrateTrigger accepts only an intensity byte.
                parameters.push(json!(8));
                parameters.push(json!(scale(*force, 255)));
            }
        }

        serde_json::to_string(&Packet {
            instructions: vec![Instruction {
                instruction_type: 1,
                parameters,
            }],
        })
        .context("failed to serialize DSX packet")
    }
}

fn scale(value: u8, maximum: u16) -> u16 {
    ((u16::from(value.min(10)) * maximum) + 5) / 10
}

#[derive(Serialize)]
struct Packet {
    instructions: Vec<Instruction>,
}

#[derive(Serialize)]
struct Instruction {
    #[serde(rename = "type")]
    instruction_type: u8,
    parameters: Vec<Value>,
}

#[cfg(test)]
mod tests {
    use std::{net::UdpSocket, time::Duration};

    use super::*;

    #[test]
    fn encodes_golden_trigger_packets() {
        let encoder = DsxPacketEncoder::new(0);
        let [left, right] = encoder
            .encode_frame(&TriggerOutputFrame {
                left: TriggerEffect::Resistance {
                    start: 10,
                    force: 10,
                },
                right: TriggerEffect::Vibrate {
                    start: 2,
                    force: 7,
                    frequency: 10,
                },
            })
            .unwrap();

        assert_eq!(
            left,
            r#"{"instructions":[{"type":1,"parameters":[0,1,13,9,8]}]}"#
        );
        assert_eq!(
            right,
            r#"{"instructions":[{"type":1,"parameters":[0,2,8,179]}]}"#
        );
    }

    #[test]
    fn encodes_normal_and_pulse() {
        let encoder = DsxPacketEncoder::new(1);
        let [left, right] = encoder
            .encode_frame(&TriggerOutputFrame {
                left: TriggerEffect::Normal,
                right: TriggerEffect::Pulse {
                    start: 2,
                    force: 6,
                    frequency: 7,
                },
            })
            .unwrap();

        assert_eq!(
            left,
            r#"{"instructions":[{"type":1,"parameters":[1,1,0]}]}"#
        );
        assert_eq!(
            right,
            r#"{"instructions":[{"type":1,"parameters":[1,2,11]}]}"#
        );
    }

    #[test]
    fn sends_two_packets_over_udp_loopback() {
        let receiver = UdpSocket::bind("127.0.0.1:0").unwrap();
        receiver
            .set_read_timeout(Some(Duration::from_secs(1)))
            .unwrap();
        let target = receiver.local_addr().unwrap();

        let mut output = DsxUdpOutput {
            socket: UdpSocket::bind("127.0.0.1:0").unwrap(),
            target,
            encoder: DsxPacketEncoder::new(0),
        };
        output
            .send(&TriggerOutputFrame {
                left: TriggerEffect::Normal,
                right: TriggerEffect::Normal,
            })
            .unwrap();

        let mut buffer = [0_u8; 256];
        let first = receiver.recv(&mut buffer).unwrap();
        let second = receiver.recv(&mut buffer).unwrap();
        assert!(first > 0);
        assert!(second > 0);
    }
}
