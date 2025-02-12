use crate::circuit::cf::Base2GateControlFunc;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::{error::Error, path::Path};

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

    pub fn evaluate(&self, x: u32) -> u32 {
        let a = (x & (1 << self.wires[1])) != 0;
        let b = (x & (1 << self.wires[2])) != 0;
        let p = self.evaluate_cf(a, b);
        x ^ ((p as u32) << self.wires[0])
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Circuit {
    pub wires: u32,
    pub gates: Vec<Gate>,
}

impl Circuit {
    pub fn random<R: Rng>(num_wires: u32, num_gates: usize, rng: &mut R) -> Self {
        let mut gates = vec![];
        for _ in 0..num_gates {
            loop {
                let target = rng.random_range(0..num_wires);
                let control_one = rng.random_range(0..num_wires);
                let control_two = rng.random_range(0..num_wires);

                if target != control_one && target != control_two && control_one != control_two {
                    gates.push(Gate {
                        wires: [target, control_one, control_two],
                        control_func: rng.random_range(0..Base2GateControlFunc::COUNT - 1),
                    });
                    break;
                }
            }
        }

        Self {
            wires: num_wires,
            gates,
        }
    }

    pub fn load_from_binary(path: impl AsRef<Path>) -> Result<Self, Box<dyn Error>> {
        Ok(bincode::deserialize(&std::fs::read(path)?)?)
    }

    pub fn save_as_binary(&self, path: impl AsRef<Path>) {
        std::fs::write(path, bincode::serialize(&self).unwrap()).unwrap();
    }

    pub fn load_from_json(path: impl AsRef<Path>) -> Self {
        serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
    }

    pub fn save_as_json(&self, path: impl AsRef<Path>) {
        std::fs::write(path, serde_json::to_vec(&self).unwrap()).unwrap();
    }

    pub fn evaluate(&self, input: u32) -> u32 {
        let mut data = input;

        self.gates.iter().for_each(|g| {
            let a = (data & (1 << g.wires[1])) != 0;
            let b = (data & (1 << g.wires[2])) != 0;
            let x = g.evaluate_cf(a, b);
            data ^= (x as u32) << g.wires[0];
        });

        data
    }
}

pub fn is_func_equiv(ckt_one: &Circuit, ckt_two: &Circuit) -> Result<(), String> {
    if ckt_one.wires != ckt_two.wires {
        return Err("Different num wires".to_string());
    }

    for i in 0..1 << ckt_one.wires {
        if ckt_one.evaluate(i) != ckt_two.evaluate(i) {
            return Err(format!(
                "Disagree on i = {:0width$b}",
                i,
                width = ckt_one.wires as usize
            ));
        }
    }

    Ok(())
}
