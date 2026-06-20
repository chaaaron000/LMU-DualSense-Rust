use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TriggerOutputFrame {
    pub left: TriggerEffect,
    pub right: TriggerEffect,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TriggerEffect {
    Normal,
    Resistance { start: u8, force: u8 },
    Pulse { start: u8, force: u8, frequency: u8 },
    Vibrate { start: u8, force: u8, frequency: u8 },
}

impl fmt::Display for TriggerEffect {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Normal => formatter.write_str("NORMAL"),
            Self::Resistance { force, .. } => write!(formatter, "RESIST({force})"),
            Self::Pulse { force, .. } => write!(formatter, "PULSE({force})"),
            Self::Vibrate { force, .. } => write!(formatter, "VIBRATE({force})"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_effects_for_live_log() {
        assert_eq!(TriggerEffect::Normal.to_string(), "NORMAL");
        assert_eq!(
            TriggerEffect::Resistance { start: 2, force: 8 }.to_string(),
            "RESIST(8)"
        );
        assert_eq!(
            TriggerEffect::Pulse {
                start: 2,
                force: 9,
                frequency: 8
            }
            .to_string(),
            "PULSE(9)"
        );
    }
}
