use crate::{local_mixing::consts::CONTROL_FUNC_TABLE, replacement::strategy::ControlFnChoice};
use rand::{seq::IndexedRandom, Rng};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Gate {
    pub wires: [usize; 3],
    pub control_func: u8,
    pub generation: usize,
}

impl Gate {
    pub fn new(target: usize, control1: usize, control2: usize, control_func: u8) -> Self {
        Self {
            wires: [target, control1, control2],
            control_func,
            generation: 0,
        }
    }

    pub fn collides_with(&self, other: &Self) -> bool {
        self.wires[0] == other.wires[1]
            || self.wires[0] == other.wires[2]
            || other.wires[0] == self.wires[1]
            || other.wires[0] == self.wires[2]
    }

    #[inline]
    pub fn evaluate(&self, input: &mut Vec<bool>) {
        let idx = ((self.control_func as usize) << 2)
            ^ ((input[self.wires[1]] as usize) << 1)
            ^ (input[self.wires[2]] as usize);
        input[self.wires[0]] ^= CONTROL_FUNC_TABLE[idx];
    }

    #[inline]
    pub fn evaluate_usize(&self, input: usize) -> usize {
        let idx = ((self.control_func as usize) << 2)
            ^ (((input >> self.wires[1]) & 1) << 1)
            ^ ((input >> self.wires[2]) & 1);
        let x = CONTROL_FUNC_TABLE[idx];
        input ^ ((x as usize) << self.wires[0])
    }
}

#[inline]
pub fn evaluate(gate_slice: &[Gate], input: &Vec<bool>) -> Vec<bool> {
    let mut bitlines = input.to_vec();
    gate_slice.iter().for_each(|g| g.evaluate(&mut bitlines));
    bitlines
}

#[inline]
pub fn evaluate_usize(gate_slice: &[Gate], input: usize) -> usize {
    let mut result = input;
    gate_slice
        .iter()
        .for_each(|g| result = g.evaluate_usize(result));
    result
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Circuit {
    pub num_wires: usize,
    pub gates: Vec<Gate>,
}

impl Circuit {
    pub fn random_with_cf<R: Rng>(
        num_wires: usize,
        num_gates: usize,
        cf_choice: ControlFnChoice,
        rng: &mut R,
    ) -> Self {
        let mut gates = vec![];
        for _ in 0..num_gates {
            loop {
                let target = rng.random_range(0..num_wires);
                let control_one = rng.random_range(0..num_wires);
                let control_two = rng.random_range(0..num_wires);

                if target != control_one && target != control_two && control_one != control_two {
                    gates.push(Gate {
                        wires: [target, control_one, control_two],
                        control_func: *cf_choice.cfs().choose(rng).unwrap(),
                        generation: 0,
                    });
                    break;
                }
            }
        }

        Self { num_wires, gates }
    }

    pub fn load_from_json(path: impl AsRef<Path>) -> Self {
        let data: CircuitData = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        Self::from(data)
    }

    pub fn save_as_json(&self, path: impl AsRef<Path>) {
        let data: CircuitData = CircuitData::from(self.clone());
        std::fs::write(path, serde_json::to_vec_pretty(&data).unwrap()).unwrap();
    }

    pub fn reset_generations(&mut self) {
        self.gates.iter_mut().for_each(|g| g.generation = 0);
    }

    pub fn save_generation_data(&self, path: impl AsRef<Path>) {
        let data = serde_json::to_vec(&self.gates.iter().map(|g| g.generation).collect::<Vec<_>>())
            .unwrap();
        std::fs::write(path, data).unwrap();
    }

    pub fn evaluate_evolution(&self, input: &Vec<bool>) -> Vec<Vec<bool>> {
        let mut data = input.clone();
        let mut evolution = vec![data.clone()];

        self.gates.iter().for_each(|g| {
            g.evaluate(&mut data);
            evolution.push(data.clone());
        });

        evolution
    }
}

pub fn check_equiv_probabilistic<R: Rng>(
    num_wires: usize,
    ckt_one: &Vec<Gate>,
    ckt_two: &Vec<Gate>,
    num_inputs: usize,
    rng: &mut R,
) -> Result<(), String> {
    if ckt_one
        .iter()
        .any(|gate| gate.wires.iter().any(|&wire| wire >= num_wires))
    {
        return Err("Wire labels in ckt_one exceed the number of wires".to_string());
    }
    if ckt_two
        .iter()
        .any(|gate| gate.wires.iter().any(|&wire| wire >= num_wires))
    {
        return Err("Wire labels in ckt_two exceed the number of wires".to_string());
    }

    let random_inputs: Vec<Vec<bool>> = (0..num_inputs)
        .map(|_| (0..num_wires).map(|_| rng.random_bool(0.5)).collect())
        .collect();

    let c1 = Circuit {
        num_wires,
        gates: ckt_one.clone(),
    };
    let c2 = Circuit {
        num_wires,
        gates: ckt_two.clone(),
    };

    random_inputs.iter().try_for_each(|random_input| {
        if evaluate(&c1.gates, random_input) != evaluate(&c2.gates, random_input) {
            return Err("Circuits produce different outputs".to_string());
        }
        Ok(())
    })
}

pub fn check_ckt_equiv_inout_map(inout_map: &[(Vec<bool>, Vec<bool>)], ckt: &[Gate]) -> bool {
    for (input, ex_output) in inout_map.iter() {
        let output = evaluate(ckt, input);
        if output != ex_output.as_slice() {
            return false;
        }
    }
    return true;
}

pub fn par_check_equiv_probabilistic<R: Rng>(
    num_wires: usize,
    ckt_one: &Vec<Gate>,
    ckt_two: &Vec<Gate>,
    num_inputs: usize,
    rng: &mut R,
) -> Result<(), String> {
    if ckt_one
        .iter()
        .any(|gate| gate.wires.iter().any(|&wire| wire >= num_wires))
    {
        return Err("Wire labels in ckt_one exceed the number of wires".to_string());
    }
    if ckt_two
        .iter()
        .any(|gate| gate.wires.iter().any(|&wire| wire >= num_wires))
    {
        return Err("Wire labels in ckt_two exceed the number of wires".to_string());
    }

    let random_inputs: Vec<Vec<bool>> = (0..num_inputs)
        .map(|_| (0..num_wires).map(|_| rng.random_bool(0.5)).collect())
        .collect();

    let c1 = Circuit {
        num_wires,
        gates: ckt_one.clone(),
    };
    let c2 = Circuit {
        num_wires,
        gates: ckt_two.clone(),
    };

    random_inputs.par_iter().try_for_each(|random_input| {
        if evaluate(&c1.gates, random_input) != evaluate(&c2.gates, random_input) {
            return Err("Circuits produce different outputs".to_string());
        }
        Ok(())
    })
}

/// Structs for saving to file

#[derive(Clone, Copy, Serialize, Deserialize, Debug)]
pub struct GateData(usize, usize, usize, u8);

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
            generation: 0,
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
            wire_count: value.num_wires,
            gate_count: value.gates.len(),
            gates: value.gates.iter().map(|g| GateData::from(*g)).collect(),
        }
    }
}

impl From<CircuitData> for Circuit {
    fn from(value: CircuitData) -> Self {
        Self {
            num_wires: value.wire_count,
            gates: value.gates.iter().map(|g| Gate::from(*g)).collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;

    use crate::circuit::circuit::par_check_equiv_probabilistic;

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
                    generation: 0,
                },
                Gate {
                    wires: [3, 0, 4],
                    control_func: 9,
                    generation: 0,
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
                    generation: 0,
                },
                Gate {
                    wires: [0, 2, 1],
                    control_func: 3,
                    generation: 0,
                },
                Gate {
                    wires: [0, 2, 1],
                    control_func: 1,
                    generation: 0,
                },
                Gate {
                    wires: [3, 0, 34],
                    control_func: 3,
                    generation: 0,
                },
            ],
        };
        let nequiv_ckt = Circuit {
            num_wires: 64,
            gates: vec![
                Gate {
                    wires: [3, 4, 0],
                    control_func: 12,
                    generation: 0,
                },
                Gate {
                    wires: [0, 2, 1],
                    control_func: 7,
                    generation: 0,
                },
                Gate {
                    wires: [0, 2, 1],
                    control_func: 1,
                    generation: 0,
                },
                Gate {
                    wires: [3, 0, 34],
                    control_func: 3,
                    generation: 0,
                },
            ],
        };
        assert!(
            par_check_equiv_probabilistic(64, &ckt.gates, &equiv_ckt.gates, 1000, &mut rng)
                == Ok(())
        );
        assert!(
            par_check_equiv_probabilistic(64, &ckt.gates, &nequiv_ckt.gates, 1000, &mut rng)
                != Ok(())
        );
    }
}
