use std::error::Error;

use crate::circuit::cf::Base2GateControlFunc;

use rand::Rng;
use serde::{Deserialize, Serialize};

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

    pub fn evaluate_cf(&self, a: bool, b: bool) -> bool {
        Base2GateControlFunc::from_u8(self.control_func).evaluate(a, b)
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

    pub fn load_from_file(path: &str) -> Result<Self, Box<dyn Error>> {
        Ok(bincode::deserialize(&std::fs::read(path)?)?)
    }

    pub fn save_to_file(&self, path: &str) {
        std::fs::write(path, bincode::serialize(&self).unwrap()).unwrap();
    }
}
