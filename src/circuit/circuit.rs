use crate::circuit::cf::Base2GateControlFunc;
use rand::Rng;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use serde::{Deserialize, Serialize};
use std::path::Path;

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

    #[inline]
    pub fn evaluate_cf(&self, a: bool, b: bool) -> bool {
        Base2GateControlFunc::from_u8(self.control_func).evaluate(a, b)
    }

    pub fn evaluate(&self, x: &mut Vec<bool>) {
        x[self.wires[0] as usize] ^=
            self.evaluate_cf(x[self.wires[1] as usize], x[self.wires[2] as usize]);
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Circuit {
    pub num_wires: u32,
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
                        control_func: rng.random_range(1..Base2GateControlFunc::COUNT),
                    });
                    break;
                }
            }
        }

        Self { num_wires, gates }
    }

    pub fn subcircuit<const SIZE: usize>(&self, index: usize) -> [Gate; SIZE] {
        std::array::from_fn(|i| self.gates[index + i])
    }

    pub fn load_from_json(path: impl AsRef<Path>) -> Self {
        let data: CircuitData = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        Self::from(data)
    }

    pub fn save_as_json(&self, path: impl AsRef<Path>) {
        let data = CircuitData::from(self.clone());
        std::fs::write(path, serde_json::to_vec_pretty(&data).unwrap()).unwrap();
    }

    pub fn evaluate(&self, input: &Vec<bool>) -> Vec<bool> {
        let mut data = input.clone();
        self.gates.iter().for_each(|g| g.evaluate(&mut data));
        data
    }
}

pub fn check_equiv_probabilistic<R: Rng>(
    ckt_one: &Circuit,
    ckt_two: &Circuit,
    num_inputs: usize,
    rng: &mut R,
) -> Result<(), String> {
    if ckt_one.num_wires != ckt_two.num_wires {
        return Err("Different num wires".to_string());
    }

    let random_inputs: Vec<Vec<bool>> = (0..num_inputs)
        .map(|_| {
            (0..ckt_one.num_wires)
                .map(|_| rng.random_bool(0.5))
                .collect()
        })
        .collect();

    random_inputs.par_iter().try_for_each(|random_input| {
        if ckt_one.evaluate(random_input) != ckt_two.evaluate(random_input) {
            return Err("Circuits produce different outputs".to_string());
        }
        Ok(())
    })
}

/// Structs for saving to file

#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct GateData(u32, u32, u32, u8);

impl From<Gate> for GateData {
    fn from(value: Gate) -> Self {
        Self(
            value.wires[1],
            value.wires[2],
            value.wires[0],
            value.control_func,
        )
    }
}

impl From<GateData> for Gate {
    fn from(value: GateData) -> Self {
        Self {
            wires: [value.2, value.0, value.1],
            control_func: value.3,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct CircuitData {
    wire_count: usize,
    gate_count: usize,
    gates: Vec<GateData>,
}

impl From<Circuit> for CircuitData {
    fn from(value: Circuit) -> Self {
        Self {
            wire_count: value.num_wires as usize,
            gate_count: value.gates.len(),
            gates: value.gates.iter().map(|g| GateData::from(*g)).collect(),
        }
    }
}

impl From<CircuitData> for Circuit {
    fn from(value: CircuitData) -> Self {
        Self {
            num_wires: value.wire_count as u32,
            gates: value.gates.iter().map(|g| Gate::from(*g)).collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;

    use crate::circuit::circuit::check_equiv_probabilistic;

    use super::{Circuit, Gate};

    #[test]
    fn test_check_equiv_probabilistic() {
        let mut rng = ChaCha8Rng::from_os_rng();
        let ckt = Circuit {
            num_wires: 64,
            gates: vec![
                Gate {
                    wires: [0, 1, 2],
                    control_func: 4,
                },
                Gate {
                    wires: [3, 0, 4],
                    control_func: 9,
                },
            ],
        };
        // Generated from find_replacement_circuit
        let equiv_ckt = Circuit {
            num_wires: 64,
            gates: vec![
                Gate {
                    wires: [3, 4, 0],
                    control_func: 12,
                },
                Gate {
                    wires: [0, 2, 1],
                    control_func: 3,
                },
                Gate {
                    wires: [0, 2, 1],
                    control_func: 1,
                },
                Gate {
                    wires: [3, 0, 34],
                    control_func: 3,
                },
            ],
        };
        let nequiv_ckt = Circuit {
            num_wires: 64,
            gates: vec![
                Gate {
                    wires: [3, 4, 0],
                    control_func: 12,
                },
                Gate {
                    wires: [0, 2, 1],
                    control_func: 7,
                },
                Gate {
                    wires: [0, 2, 1],
                    control_func: 1,
                },
                Gate {
                    wires: [3, 0, 34],
                    control_func: 3,
                },
            ],
        };
        assert!(check_equiv_probabilistic(&ckt, &equiv_ckt, 1000, &mut rng) == Ok(()));
        assert!(check_equiv_probabilistic(&ckt, &nequiv_ckt, 1000, &mut rng) != Ok(()));
    }
}
