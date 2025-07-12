use serde::{Deserialize, Serialize};

use crate::circuit::analysis::{
    num_distinct_wires, optimal_projection_circuit, projection_circuit,
};
use crate::circuit::circuit::evaluate_usize;
use crate::circuit::{cf::GateLibrary, Gate};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Deserialize)]
pub struct OldCompressionTable {
    pub max_gates_supported: usize,
    pub max_wires_supported: usize,
    pub gate_library: GateLibrary,
    pub table: HashMap<Vec<usize>, Vec<Gate>>,
}

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct CompressionTable {
    pub max_gates_supported: usize,
    pub max_wires_supported: usize,
    pub gate_library: GateLibrary,
    pub table: HashMap<Vec<usize>, Vec<Vec<Gate>>>,
    // TODO:
    // pub one_ckt_per_tt: bool,
    // pub use_canonicalized_ckts: bool,
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
            table: build_compression_table_better(
                max_gates_supported,
                max_wires_supported,
                gate_library,
                true,
                true,
            ),
            gate_library,
        }
    }

    pub fn empty() -> Self {
        Self {
            max_gates_supported: 0,
            max_wires_supported: 0,
            gate_library: GateLibrary::All,
            table: HashMap::new(),
        }
    }

    pub fn from_file(path: &str) -> Self {
        let data = std::fs::read(path).expect("Failed to read file");
        match bincode::deserialize::<Self>(&data) {
            Ok(table) => table,
            Err(_) => {
                let old_table: OldCompressionTable = bincode::deserialize(&data)
                    .expect("Failed to deserialize old compression table format");
                Self {
                    max_gates_supported: old_table.max_gates_supported,
                    max_wires_supported: old_table.max_wires_supported,
                    gate_library: old_table.gate_library,
                    table: old_table
                        .table
                        .into_iter()
                        .map(|(k, v)| (k, vec![v]))
                        .collect(),
                }
            }
        }
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
        Some(self.table.get(tt)?[0].len())
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
        let mut output = res[0].clone();

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
        let mut output = res[0].clone();

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

pub fn build_compression_table_even_better(
    max_gates_supported: usize,
    max_wires_supported: usize,
    gate_library: GateLibrary,
) -> HashMap<Vec<usize>, usize> {
    assert!(max_gates_supported >= 1);
    assert!(max_wires_supported >= 3);

    let tt_size = 1 << max_wires_supported;

    let mut table = HashMap::new();
    // Identity gate
    table.insert((0..tt_size).collect(), 0);

    let mut current_layer: Vec<Vec<usize>> = vec![(0..tt_size).collect()];

    for cxity in 1..max_gates_supported {
        println!(
            "{} truth tables of new cxity. Processing {} candidates now",
            current_layer.len(),
            current_layer.len()
                * gate_library.cfs().len()
                * max_wires_supported
                * (max_wires_supported - 1)
                * (max_wires_supported - 2)
                / 2
        );
        let mut next_layer = vec![];

        for t in 0..max_wires_supported {
            for c1 in 0..max_wires_supported {
                if t != c1 {
                    for c2 in c1 + 1..max_wires_supported {
                        if t != c2 {
                            for cf in gate_library.cfs() {
                                let mut num_added = 0;
                                let next_gate = Gate {
                                    wires: [t, c1, c2],
                                    control_func: cf,
                                    generation: 0,
                                };
                                for tt in &current_layer {
                                    let next_tt: Vec<usize> =
                                        tt.iter().map(|&x| next_gate.evaluate_usize(x)).collect();
                                    match table.entry(next_tt.clone()) {
                                        std::collections::hash_map::Entry::Vacant(e) => {
                                            e.insert(cxity);
                                            next_layer.push(next_tt);
                                            num_added += 1;
                                        }
                                        std::collections::hash_map::Entry::Occupied(_) => {}
                                    }
                                }
                                println!(
                                    "cxity = {}, num added = {}, gate = {:?}",
                                    cxity, num_added, next_gate
                                );
                            }
                        }
                    }
                }
            }
        }

        current_layer = next_layer;
    }

    println!(
        "{} truth tables of cxity = {}. Processing {} candidates for cxity = {}",
        current_layer.len(),
        max_gates_supported - 1,
        current_layer.len()
            * max_wires_supported
            * (max_wires_supported - 1)
            * (max_wires_supported - 2)
            / 2,
        max_gates_supported,
    );

    // Max cxity
    for t in 0..max_wires_supported {
        for c1 in 0..max_wires_supported {
            if t != c1 {
                for c2 in c1 + 1..max_wires_supported {
                    if t != c2 {
                        for cf in gate_library.cfs() {
                            let mut num_added = 0;
                            let next_gate = Gate {
                                wires: [t, c1, c2],
                                control_func: cf,
                                generation: 0,
                            };
                            for tt in &current_layer {
                                let next_tt: Vec<usize> =
                                    tt.iter().map(|&x| next_gate.evaluate_usize(x)).collect();
                                match table.entry(next_tt.clone()) {
                                    std::collections::hash_map::Entry::Vacant(e) => {
                                        e.insert(max_gates_supported);
                                        num_added += 1;
                                    }
                                    std::collections::hash_map::Entry::Occupied(_) => {}
                                }
                            }
                            println!(
                                "cxity = {}, num added = {}, gate = {:?}",
                                max_gates_supported, num_added, next_gate
                            );
                        }
                    }
                }
            }
        }
    }

    table
}

pub fn build_compression_table_better(
    max_gates_supported: usize,
    max_wires_supported: usize,
    gate_library: GateLibrary,
    one_ckt_per_tt: bool,
    use_canonicalized_ckts: bool,
) -> HashMap<Vec<usize>, Vec<Vec<Gate>>> {
    assert!(max_gates_supported >= 1);
    assert!(max_wires_supported >= 3);

    let tt_size = 1 << max_wires_supported;

    let mut table = HashMap::new();
    // Identity gate
    table.insert((0..tt_size).collect(), vec![vec![]]);

    let mut current_layer: Vec<(Vec<Gate>, Vec<usize>, usize)> =
        vec![(vec![], (0..tt_size).collect(), 0)];

    for num_gates in 1..=max_gates_supported {
        println!(
            "{} circuits in last layer. Processing ckts of size {}",
            current_layer.len(),
            num_gates
        );
        let mut next_layer: Vec<(Vec<Gate>, Vec<usize>, usize)> = vec![];

        for (ckt, curr_tt, num_distinct_wires) in &current_layer {
            let next_bitline = *num_distinct_wires;
            if use_canonicalized_ckts {
                let mut next_wire_set = vec![vec![]];

                // 0 added wires
                for t in 0..next_bitline {
                    for c1 in 0..next_bitline {
                        if t != c1 {
                            for c2 in c1 + 1..next_bitline {
                                if t != c2 {
                                    next_wire_set[0].push([t, c1, c2]);
                                }
                            }
                        }
                    }
                }
                next_wire_set.push(vec![]);
                next_wire_set.push(vec![]);
                next_wire_set.push(vec![]);

                // 1 added wire
                if next_bitline < max_wires_supported {
                    for b1 in 0..next_bitline {
                        for b2 in 0..next_bitline {
                            if b1 != b2 {
                                next_wire_set[1].push([next_bitline, b1, b2]);
                                next_wire_set[1].push([b1, next_bitline, b2]);
                                next_wire_set[1].push([b1, b2, next_bitline]);
                            }
                        }
                    }
                }

                // 2 added wires
                if next_bitline < max_wires_supported - 1 {
                    for b1 in 0..next_bitline {
                        next_wire_set[2].push([b1, next_bitline, next_bitline + 1]);
                        next_wire_set[2].push([next_bitline, b1, next_bitline + 1]);
                        next_wire_set[2].push([next_bitline, next_bitline + 1, b1]);
                    }
                }

                // 3 added wires
                if next_bitline < max_wires_supported - 2 {
                    next_wire_set[3].push([next_bitline, next_bitline + 1, next_bitline + 2]);
                }

                for num_added_wires in 0..=3 {
                    next_wire_set[num_added_wires].iter().for_each(|wires| {
                        for cf in gate_library.cfs() {
                            let next_gate = Gate {
                                wires: *wires,
                                control_func: cf,
                                generation: 0,
                            };
                            let next_tt: Vec<usize> = curr_tt
                                .iter()
                                .map(|&x| next_gate.evaluate_usize(x))
                                .collect();
                            let mut next_ckt = ckt.clone();
                            next_ckt.push(next_gate);
                            table
                                .entry(next_tt.clone())
                                .and_modify(|list| {
                                    if !one_ckt_per_tt {
                                        list.push(next_ckt.clone());
                                    }
                                })
                                .or_insert(vec![next_ckt.clone()]);
                            next_layer.push((
                                next_ckt,
                                next_tt,
                                *num_distinct_wires + num_added_wires,
                            ));
                        }
                    });
                }
            } else {
                for t in 0..max_wires_supported {
                    for c1 in 0..max_wires_supported {
                        if t != c1 {
                            for c2 in c1 + 1..max_wires_supported {
                                if t != c2 {
                                    for cf in gate_library.cfs() {
                                        let next_gate = Gate {
                                            wires: [t, c1, c2],
                                            control_func: cf,
                                            generation: 0,
                                        };
                                        let next_tt: Vec<usize> = curr_tt
                                            .iter()
                                            .map(|&x| next_gate.evaluate_usize(x))
                                            .collect();
                                        let mut next_ckt = ckt.clone();
                                        next_ckt.push(next_gate);
                                        table
                                            .entry(next_tt.clone())
                                            .and_modify(|list| {
                                                if !one_ckt_per_tt {
                                                    list.push(next_ckt.clone());
                                                }
                                            })
                                            .or_insert(vec![next_ckt.clone()]);
                                        next_layer.push((next_ckt, next_tt, 0));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        if num_gates != max_gates_supported {
            current_layer = next_layer;
        }
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
    use crate::{
        circuit::{
            analysis::{optimal_projection_circuit, truth_table},
            cf::GateLibrary,
            Circuit,
        },
        compression::ct::build_compression_table_even_better,
    };

    #[test]
    fn test_compression_table() {
        let gates = 4;
        let wires = 6;
        let gate_library = GateLibrary::TwoBit;
        let s = Instant::now();
        let ct = CompressionTable::new(gates, wires, gate_library);
        let d = Instant::now() - s;
        dbg!(d);

        let mut rng = rand::rng();
        for _ in 0..1000000 {
            let circuit =
                Circuit::random_with_cf(wires, gates, GateLibrary::TwoBit, &mut rng).gates;
            let (proj_circuit, _, _) = optimal_projection_circuit(&circuit);
            let tt = truth_table(wires, &proj_circuit);
            let res = ct.lookup_truth_table(&tt);
            if res.is_none() {
                dbg!(&circuit);
                dbg!(&proj_circuit);
            }
            assert!(res.is_some());
        }

        ct.save_to_file("test.db");
    }

    #[test]
    fn test_build_compression_table_even_better() {
        let num_wires = 6;
        let num_gates = 3;
        let gate_library = GateLibrary::TwoBit;
        let s = Instant::now();
        let table = build_compression_table_even_better(num_gates, num_wires, gate_library);
        let d = Instant::now() - s;
        dbg!(d);
        dbg!(table.len());
    }
}
