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
