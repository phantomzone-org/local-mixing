use crate::circuit::cf::Base2GateControlFunc;
use rand::Rng;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
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
                        control_func: rng.random_range(0..Base2GateControlFunc::COUNT - 1),
                    });
                    break;
                }
            }
        }

        Self { num_wires, gates }
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
        std::fs::write(path, serde_json::to_vec_pretty(&self).unwrap()).unwrap();
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

#[derive(Serialize, Deserialize)]
pub struct PrettyCircuit {
    wire_count: usize,
    gate_count: usize,
    gates: Vec<[u32; 4]>,
}

impl PrettyCircuit {
    pub fn save_as_json(&self, path: impl AsRef<Path>) {
        std::fs::write(path, serde_json::to_vec_pretty(&self).unwrap()).unwrap();
    }
}

impl From<Circuit> for PrettyCircuit {
    fn from(c: Circuit) -> Self {
        Self {
            wire_count: c.num_wires as usize,
            gate_count: c.gates.len(),
            gates: c
                .gates
                .iter()
                .map(|gate| {
                    [
                        gate.wires[0],
                        gate.wires[1],
                        gate.wires[2],
                        gate.control_func.into(),
                    ]
                })
                .collect(),
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
