use rand::{seq::IndexedRandom, Rng};
use serde::{Deserialize, Serialize};

use crate::circuit::cf::Base2GateControlFunc;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ReplacementStrategy {
    SampleUnguided,
    SampleActive0,
    Dummy,
}

impl ReplacementStrategy {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::SampleUnguided),
            1 => Some(Self::SampleActive0),
            2 => Some(Self::Dummy),
            _ => None,
        }
    }
}

impl Default for ReplacementStrategy {
    fn default() -> Self {
        Self::SampleActive0
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ControlFnChoice {
    All,
    NoIdentity,
    OnlyUnique,
    UniqueNo0Bit,
}

impl ControlFnChoice {
    #[inline]
    pub fn random_cf<R: Rng>(&self, rng: &mut R) -> u8 {
        match self {
            Self::All => rng.random_range(0..Base2GateControlFunc::COUNT),
            Self::NoIdentity => rng.random_range(1..Base2GateControlFunc::COUNT),
            Self::OnlyUnique => *[15, 3, 12, 1, 4, 7, 13, 6, 9, 14, 8].choose(rng).unwrap(),
            Self::UniqueNo0Bit => *[3, 12, 1, 4, 7, 13, 6, 9, 14, 8].choose(rng).unwrap(),
        }
    }

    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::All),
            1 => Some(Self::NoIdentity),
            2 => Some(Self::OnlyUnique),
            _ => None,
        }
    }
}

impl Default for ControlFnChoice {
    fn default() -> Self {
        Self::OnlyUnique
    }
}
