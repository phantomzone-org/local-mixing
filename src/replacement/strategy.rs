use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ReplacementStrategy {
    SampleUnguided,
    SampleActive0,
    SampleActive1,
    Dummy,
}

impl ReplacementStrategy {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::SampleUnguided),
            1 => Some(Self::SampleActive0),
            2 => Some(Self::SampleActive1),
            3 => Some(Self::Dummy),
            _ => None,
        }
    }
}

impl Default for ReplacementStrategy {
    fn default() -> Self {
        Self::SampleActive0
    }
}
