use serde::{Deserialize, Serialize};

use crate::circuit::analysis::{
    num_distinct_wires, optimal_projection_circuit, projection_circuit,
};
use crate::circuit::circuit::evaluate_usize;
use crate::circuit::{cf::GateLibrary, Gate};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct CompressionTable {
    pub max_gates_supported: usize,
    pub max_wires_supported: usize,
    pub gate_library: GateLibrary,
    pub table: HashMap<Vec<usize>, Vec<Gate>>,
}

impl CompressionTable {
    pub fn new(
        max_gates_supported: usize,
        max_wires_supported: usize,
        gate_library: GateLibrary,
    ) -> Self {
        Self {
            max_gates_supported,
            max_wires_supported,
            table: build_compression_table(max_gates_supported, max_wires_supported, gate_library),
            gate_library,
        }
    }

    pub fn from_file(path: &str) -> Self {
        let data = std::fs::read(path).expect("Failed to read file");
        bincode::deserialize(&data).expect("Failed to deserialize compression table")
    }

    pub fn save_to_file(&self, path: &str) {
        let path_obj = Path::new(path);
        if let Some(parent_dir) = path_obj.parent() {
            fs::create_dir_all(parent_dir).expect("Failed to create directory");
        }
        let data = bincode::serialize(self).expect("Failed to serialize compression table");
        fs::write(path, data).expect("Failed to write file");
    }

    pub fn lookup_truth_table(&self, tt: &Vec<usize>) -> Option<usize> {
        Some(self.table.get(tt)?.len())
    }

    pub fn compress(&self, circuit_gates: &[Gate]) -> Option<Vec<Gate>> {
        let (proj_circuit, proj_map) = projection_circuit(circuit_gates);
        let num_wires = num_distinct_wires(&proj_circuit);
        if num_wires > self.max_wires_supported {
            return None;
        }

        let tt: Vec<_> = (0..1 << self.max_wires_supported)
            .map(|x| evaluate_usize(&proj_circuit, x))
            .collect();
        let res = self.table.get(&tt)?;
        if res.len() >= circuit_gates.len() {
            return None;
        }
        let mut output = res.clone();

        for g in output.iter_mut() {
            for i in 0..3 {
                if g.wires[i] < proj_map.len() {
                    g.wires[i] = proj_map[g.wires[i]];
                } else {
                    g.wires[i] = 0;
                }
            }
        }

        Some(output)
    }

    pub fn compress_with_optimal_relabel(&self, circuit_gates: &[Gate]) -> Option<Vec<Gate>> {
        let (proj_circuit, proj_map, num_active_wires) = optimal_projection_circuit(circuit_gates);
        if num_active_wires > self.max_wires_supported {
            return None;
        }

        let tt: Vec<_> = (0..1 << self.max_wires_supported)
            .map(|x| evaluate_usize(&proj_circuit, x))
            .collect();
        let res = self.table.get(&tt)?;
        if res.len() >= circuit_gates.len() {
            return None;
        }
        let mut output = res.clone();

        for g in output.iter_mut() {
            for i in 0..3 {
                if g.wires[i] < proj_map.len() {
                    g.wires[i] = proj_map[g.wires[i]];
                } else {
                    g.wires[i] = 0;
                }
            }
        }

        Some(output)
    }
}

pub fn build_compression_table(
    max_gates_supported: usize,
    max_wires_supported: usize,
    gate_library: GateLibrary,
) -> HashMap<Vec<usize>, Vec<Gate>> {
    assert!(max_gates_supported >= 1);
    assert!(max_wires_supported >= 3);

    let tt_size = 1 << max_wires_supported;

    let mut table = HashMap::new();
    // Identity gate
    table.insert((0..tt_size).collect(), vec![]);

    let mut current_circuit = vec![Gate::default(); max_gates_supported];
    current_circuit[0].wires = [0, 1, 2];

    for cf in gate_library.cfs() {
        current_circuit[0].control_func = cf;
        build_compression_table_recursive(
            max_gates_supported,
            max_wires_supported,
            gate_library,
            &mut current_circuit,
            &(0..tt_size).collect(),
            1,
            3,
            &mut table,
        );
    }

    table
}

fn build_compression_table_recursive(
    max_gates_supported: usize,
    max_wires_supported: usize,
    gate_library: GateLibrary,
    current_circuit: &mut Vec<Gate>,
    current_tt: &Vec<usize>,
    current_size: usize,
    wires_used: usize,
    table: &mut HashMap<Vec<usize>, Vec<Gate>>,
) {
    if wires_used > max_wires_supported {
        return;
    }

    let tt: Vec<usize> = current_tt
        .iter()
        .map(|&x| current_circuit[current_size - 1].evaluate_usize(x))
        .collect();

    table
        .entry(tt.clone())
        .and_modify(|e| {
            if current_size < e.len() {
                *e = current_circuit[0..current_size].to_vec()
            }
        })
        .or_insert(current_circuit[0..current_size].to_vec());

    if current_size == max_gates_supported || wires_used == max_wires_supported {
        return;
    }

    for cf in gate_library.cfs() {
        current_circuit[current_size].control_func = cf;

        // Three new wires
        current_circuit[current_size].wires = [wires_used, wires_used + 1, wires_used + 2];
        build_compression_table_recursive(
            max_gates_supported,
            max_wires_supported,
            gate_library,
            current_circuit,
            &tt,
            current_size + 1,
            wires_used + 3,
            table,
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
                    gate_library,
                    current_circuit,
                    &tt,
                    current_size + 1,
                    wires_used + 2,
                    table,
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
                            gate_library,
                            current_circuit,
                            &tt,
                            current_size + 1,
                            wires_used + 1,
                            table,
                        );

                        // Three old wires
                        for label3 in 0..wires_used {
                            if label3 != label1 && label3 != label2 {
                                current_circuit[current_size].wires[w] = label3;
                                build_compression_table_recursive(
                                    max_gates_supported,
                                    max_wires_supported,
                                    gate_library,
                                    current_circuit,
                                    &tt,
                                    current_size + 1,
                                    wires_used,
                                    table,
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

    use std::time::Instant;

    use super::CompressionTable;
    use crate::circuit::{
        analysis::{projection_circuit, truth_table},
        cf::GateLibrary,
        Circuit,
    };

    #[test]
    fn test_compression_table() {
        let gates = 3;
        let wires = 9;
        let gate_library = GateLibrary::TwoBit;
        let s = Instant::now();
        let ct = CompressionTable::new(gates, wires, gate_library);
        let d = Instant::now() - s;
        dbg!(d);

        let mut rng = rand::rng();
        for _ in 0..1000000 {
            let circuit =
                Circuit::random_with_cf(wires, gates, GateLibrary::TwoBit, &mut rng).gates;
            let tt = truth_table(wires, &circuit);
            let res = ct.lookup_truth_table(&tt);
            if res.is_none() {
                dbg!(&circuit);
                let proj_circuit = projection_circuit(&circuit).0;
                dbg!(&proj_circuit);
            }
            assert!(res.is_some());
        }
    }
}
