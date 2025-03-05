#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Base2GateControlFunc {
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

impl Base2GateControlFunc {
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
}
