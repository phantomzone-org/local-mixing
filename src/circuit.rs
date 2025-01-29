use rand::Rng;
use serde::{Deserialize, Serialize};

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Base2GateControlFunc {
    T = 0,     // true,
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
               // F = 15,    // false,
}

impl Base2GateControlFunc {
    pub const COUNT: u8 = 15;
    pub const fn from_u8(v: u8) -> Self {
        match v {
            0 => Self::T,
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
            // 15 => Self::F,
            _ => unreachable!(),
        }
    }

    pub const fn evaluate(&self, a: bool, b: bool) -> bool {
        match self {
            // Self::F => false,
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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Gate {
    pub wires: [u32; 3],
    pub control_func: u8,
}

impl Gate {
    pub fn new(target: u32, control1: u32, control2: u32, control_func: u8) -> Self {
        Self {
            wires: [target, control1, control2],
            control_func,
        }
    }

    pub fn collides_with(&self, other: &Self) -> bool {
        self.wires[0] == other.wires[1]
            || self.wires[0] == other.wires[2]
            || other.wires[0] == self.wires[1]
            || other.wires[0] == self.wires[2]
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Circuit {
    pub gates: Vec<Gate>,
}

impl Circuit {
    pub fn random<R: Rng>(num_wires: u32, num_gates: usize, rng: &mut R) -> Self {
        let mut gates = vec![];
        for _ in 0..num_gates {
            loop {
                let target = rng.gen_range(0..num_wires);
                let control_one = rng.gen_range(0..num_wires);
                let control_two = rng.gen_range(0..num_wires);

                if target != control_one && target != control_two && control_one != control_two {
                    gates.push(Gate {
                        wires: [target, control_one, control_two],
                        control_func: rng.gen_range(0..Base2GateControlFunc::COUNT),
                    });
                    break;
                }
            }
        }

        Self { gates }
    }
}

#[cfg(test)]
mod tests {
    use rand::thread_rng;

    use super::*;

    #[test]
    fn test_random_circuit() {
        let mut rng = thread_rng();
        let circuit = Circuit::random(10, 50, &mut rng);
        assert!(circuit.gates.len() == 50);
    }
}
