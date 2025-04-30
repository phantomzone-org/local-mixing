use std::error::Error;

use serde::{Deserialize, Serialize};

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GateControlFunc {
    F = 0,     // false,
    AND = 1,   // a & b,
    ANDNB = 2, // a & (!b),
    A = 3,     // a,
    ANDNA = 4, // (!a) & b,
    B = 5,     // b,
    XOR = 6,   // a ^ b,
    OR = 7,    // a | b,
    NOR = 8,   // !(a | b),
    EQUIV = 9, // (a & b) | ((!a) & (!b)),
    NB = 10,   // !b,
    ORNB = 11, // (!b) | a,
    NA = 12,   // !a,
    ORNA = 13, // (!a) | b,
    NAND = 14, // !(a & b),
    T = 15,    // true,
}

impl GateControlFunc {
    pub const COUNT: u8 = 16;
    pub const fn from_u8(v: u8) -> Self {
        match v {
            0 => Self::F,
            1 => Self::AND,
            2 => Self::ANDNB,
            3 => Self::A,
            4 => Self::ANDNA,
            5 => Self::B,
            6 => Self::XOR,
            7 => Self::OR,
            8 => Self::NOR,
            9 => Self::EQUIV,
            10 => Self::NB,
            11 => Self::ORNB,
            12 => Self::NA,
            13 => Self::ORNA,
            14 => Self::NAND,
            15 => Self::T,
            _ => unreachable!(),
        }
    }

    pub const fn evaluate(&self, a: bool, b: bool) -> bool {
        match self {
            Self::F => false,
            Self::AND => a & b,
            Self::ANDNB => a & (!b),
            Self::A => a,
            Self::ANDNA => (!a) & b,
            Self::B => b,
            Self::XOR => a ^ b,
            Self::OR => a | b,
            Self::NOR => !(a | b),
            Self::EQUIV => (a & b) | ((!a) & (!b)),
            Self::NB => !b,
            Self::ORNB => (!b) | a,
            Self::NA => !a,
            Self::ORNA => (!a) | b,
            Self::NAND => !(a & b),
            Self::T => true,
        }
    }

    pub const fn opposite_on_controls(v: u8) -> u8 {
        match v {
            2 => 4,   // ANDNA
            3 => 5,   // B
            4 => 2,   // ANDNB
            5 => 3,   // A
            10 => 12, // NA
            11 => 13, // ORNA
            12 => 10, // NB
            13 => 11, // ORNB
            _ => v,
        }
    }

    pub const fn negated(v: u8) -> u8 {
        match v {
            0 => 15, // F -> T
            1 => 14, // AND -> NAND
            2 => 13, // ANDNB -> ORNA
            3 => 12, // A -> NA
            4 => 11, // ANDNA -> ORNB
            5 => 10, // B -> NB
            6 => 9,  // XOR -> EQUIV
            7 => 8,  // OR -> NOR
            8 => 7,  // NOR -> OR
            9 => 6,  // EQUIV -> XOR
            10 => 5, // NB -> B
            11 => 4, // ORNB -> ANDNA
            12 => 3, // NA -> A
            13 => 2, // ORNA -> ANDNB
            14 => 1, // NAND -> AND
            15 => 0, // T -> F
            _ => unreachable!(),
        }
    }

    pub fn to_string(&self) -> String {
        match self {
            Self::F => "0".to_string(),
            Self::AND => "a&b".to_string(),
            Self::ANDNB => "a&!b".to_string(),
            Self::A => "a".to_string(),
            Self::ANDNA => "!a&b".to_string(),
            Self::B => "b".to_string(),
            Self::XOR => "a^b".to_string(),
            Self::OR => "a|b".to_string(),
            Self::NOR => "!(a|b)".to_string(),
            Self::EQUIV => "a=b".to_string(),
            Self::NB => "!b".to_string(),
            Self::ORNB => "!b|a".to_string(),
            Self::NA => "!a".to_string(),
            Self::ORNA => "!a|b".to_string(),
            Self::NAND => "!(a&b)".to_string(),
            Self::T => "1".to_string(),
        }
    }
}

#[derive(Clone, Copy, Default, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum GateLibrary {
    #[default]
    All,
    NoIdentity,
    OnlyUnique,
    UniqueNo0Bit,
    TwoBit,
}

impl GateLibrary {
    pub fn cfs(&self) -> Vec<u8> {
        match self {
            Self::All => (0..GateControlFunc::COUNT).collect(),
            Self::NoIdentity => (1..GateControlFunc::COUNT).collect(),
            Self::OnlyUnique => vec![15, 3, 12, 1, 4, 7, 13, 6, 9, 14, 8],
            Self::UniqueNo0Bit => vec![3, 12, 1, 4, 7, 13, 6, 9, 14, 8],
            Self::TwoBit => vec![1, 2, 4, 6, 7, 8, 9, 11, 13, 14],
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

    pub fn from_str(raw_gate_library: &str) -> Result<Self, Box<dyn Error>> {
        match raw_gate_library {
            "All" => Ok(Self::All),
            "NoIdentity" => Ok(Self::NoIdentity),
            "OnlyUnique" => Ok(Self::OnlyUnique),
            "UniqueNo0Bit" => Ok(Self::UniqueNo0Bit),
            "TwoBit" => Ok(Self::TwoBit),
            _ => Err(Box::<dyn Error>::from(format!(
                "Cannot parse '{}'",
                raw_gate_library
            ))),
        }
    }
}
