use serde::{Deserialize, Serialize};

use crate::circuit::{
    analysis::{optimal_projection_circuit, truth_table},
    cf::Base2GateControlFunc,
    Gate,
};
use std::collections::HashMap;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CompressionTable {
    pub max_gates_supported: usize,
    pub max_wires_supported: usize,
    pub cf_choice: Vec<u8>,
    ct: HashMap<Vec<usize>, Vec<Gate>>,
    #[serde(skip_serializing, skip_deserializing)]
    cache: HashMap<Vec<Gate>, Vec<Gate>>,
}

impl CompressionTable {
    pub fn new(max_gates_supported: usize, max_wires_supported: usize, cf_choice: Vec<u8>) -> Self {
        Self {
            max_gates_supported,
            max_wires_supported,
            ct: build_compression_table(max_gates_supported, max_wires_supported, &cf_choice),
            cf_choice,
            cache: HashMap::new(),
        }
    }

    pub fn from_file(path: &str) -> Self {
        let data = std::fs::read(path).expect("Failed to read file");
        bincode::deserialize(&data).expect("Failed to deserialize compression table")
    }

    pub fn save_to_file(&self, path: &str) {
        let data = bincode::serialize(self).expect("Failed to serialize compression table");
        std::fs::write(path, data).expect("Failed to write file");
    }

    pub fn lookup_cxity(&self, circuit: &Vec<Gate>) -> Option<usize> {
        if let Some(saved) = self.cache.get(circuit) {
            return Some(saved.len());
        }
        let (proj_circuit, _, _, num_active_wires) = optimal_projection_circuit(circuit);
        if num_active_wires > self.max_wires_supported {
            return None;
        }

        let truth_table = truth_table(self.max_wires_supported, &proj_circuit);

        // let match_circuit = self.ct.get(&truth_table.to_vec())?;
        Some(self.ct.get(&truth_table)?.len())
    }

    pub fn compress_circuit(&mut self, circuit: &Vec<Gate>) -> Option<Vec<Gate>> {
        if let Some(saved) = self.cache.get(circuit) {
            return Some(saved.to_vec());
        }
        let (proj_circuit, proj_map, _, num_active_wires) = optimal_projection_circuit(circuit);
        if num_active_wires > self.max_wires_supported {
            return None;
        }

        let truth_table = truth_table(self.max_wires_supported, &proj_circuit);

        let match_circuit = self.ct.get(&truth_table.to_vec())?;

        let output_circuit: Vec<Gate> = match_circuit
            .iter()
            .map(|g| {
                let mut wires = [0; 3];
                for i in 0..3 {
                    let match_wire = g.wires[i];
                    wires[i] = if match_wire < proj_map.len() {
                        proj_map[match_wire]
                    } else {
                        // TODO: Fix. [0, 0, 0], cf = 5 will not be reversible
                        0
                    };
                }
                Gate {
                    wires,
                    control_func: g.control_func,
                    generation: 0,
                }
            })
            .collect();

        self.cache.insert(circuit.to_vec(), output_circuit.clone());
        Some(output_circuit)
    }
}

pub fn build_compression_table(
    max_gates_supported: usize,
    max_wires_supported: usize,
    cf_choice: &Vec<u8>,
) -> HashMap<Vec<usize>, Vec<Gate>> {
    assert!(max_gates_supported >= 1);
    assert!(max_wires_supported >= 3);

    let tt_size = 1 << max_wires_supported;

    let mut ct = HashMap::new();
    // Identity gate
    ct.insert((0..tt_size).collect(), vec![]);

    let mut current_circuit = vec![Gate::default(); max_gates_supported];
    current_circuit[0].wires = [0, 1, 2];

    for cf in cf_choice {
        current_circuit[0].control_func = *cf;
        build_compression_table_recursive(
            max_gates_supported,
            max_wires_supported,
            cf_choice,
            1,
            &mut current_circuit,
            3,
            &mut ct,
        );
    }

    ct
}

fn build_compression_table_recursive(
    max_gates_supported: usize,
    max_wires_supported: usize,
    cf_choice: &Vec<u8>,
    current_size: usize,
    current_circuit: &mut Vec<Gate>,
    wires_used: usize,
    ct: &mut HashMap<Vec<usize>, Vec<Gate>>,
) {
    if wires_used > max_wires_supported {
        return;
    }

    let tt = (0..1 << max_wires_supported)
        .map(|i| {
            let mut input = i;
            current_circuit.iter().take(current_size).for_each(|g| {
                let a = (input & (1 << g.wires[1])) != 0;
                let b = (input & (1 << g.wires[2])) != 0;
                let x = g.evaluate_cf(a, b);
                input ^= (x as usize) << g.wires[0];
            });
            input
        })
        .collect();

    ct.entry(tt)
        .and_modify(|e| {
            if current_size < e.len() {
                *e = current_circuit[0..current_size].to_vec()
            }
        })
        .or_insert(current_circuit[0..current_size].to_vec());

    if current_size == max_gates_supported || wires_used == max_wires_supported {
        return;
    }

    for cf in 1..Base2GateControlFunc::COUNT {
        current_circuit[current_size].control_func = cf;

        // Three new wires
        current_circuit[current_size].wires = [wires_used, wires_used + 1, wires_used + 2];
        build_compression_table_recursive(
            max_gates_supported,
            max_wires_supported,
            cf_choice,
            current_size + 1,
            current_circuit,
            wires_used + 3,
            ct,
        );

        for w in 0..3 {
            let other_wires = other_two_wire_pos(w);
            // Two new wires: w is an old wire
            current_circuit[current_size].wires[other_wires[0]] = wires_used;
            current_circuit[current_size].wires[other_wires[1]] = wires_used + 1;
            for label in 0..wires_used {
                current_circuit[current_size].wires[w] = label;
                build_compression_table_recursive(
                    max_gates_supported,
                    max_wires_supported,
                    cf_choice,
                    current_size + 1,
                    current_circuit,
                    wires_used + 2,
                    ct,
                );
            }

            for label1 in 0..wires_used {
                for label2 in 0..wires_used {
                    if label1 != label2 {
                        // Two old wires: w is new
                        current_circuit[current_size].wires[w] = wires_used;
                        current_circuit[current_size].wires[other_wires[0]] = label1;
                        current_circuit[current_size].wires[other_wires[1]] = label2;
                        build_compression_table_recursive(
                            max_gates_supported,
                            max_wires_supported,
                            cf_choice,
                            current_size + 1,
                            current_circuit,
                            wires_used + 1,
                            ct,
                        );

                        // Three old wires
                        for label3 in 0..wires_used {
                            if label3 != label1 && label3 != label2 {
                                current_circuit[current_size].wires[w] = label3;
                                build_compression_table_recursive(
                                    max_gates_supported,
                                    max_wires_supported,
                                    cf_choice,
                                    current_size + 1,
                                    current_circuit,
                                    wires_used,
                                    ct,
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}

const fn other_two_wire_pos(wire_pos: usize) -> [usize; 2] {
    match wire_pos {
        0 => [1, 2],
        1 => [0, 2],
        2 => [0, 1],
        _ => panic!(),
    }
}

#[cfg(test)]
mod tests {

    use super::CompressionTable;
    use crate::{circuit::Circuit, replacement::strategy::ControlFnChoice};

    #[test]
    fn test_ct_real() {
        let mut ct = CompressionTable::new(3, 9, ControlFnChoice::OnlyUnique.cfs());

        let mut rng = rand::rng();
        for _ in 0..10000 {
            let circuit = Circuit::random_with_cf(9, 3, &ControlFnChoice::OnlyUnique.cfs(), &mut rng).gates;

            let res = ct.compress_circuit(&circuit);
            assert!(res.is_some());
        }

        ct.save_to_file("bin/table.db");
    }
}
