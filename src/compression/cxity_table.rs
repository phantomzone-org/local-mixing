use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::circuit::{cf::GateLibrary, Gate};

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct PermutationComplexityTable {
    pub max_cxity: usize,
    pub max_wires: usize,
    pub gate_library: GateLibrary,
    pub table: HashMap<Vec<usize>, usize>,
}

impl PermutationComplexityTable {
    pub fn new(max_cxity: usize, max_wires: usize, gate_library: GateLibrary) -> Self {
        Self {
            max_cxity,
            max_wires,
            gate_library,
            table: build_complexity_table(max_cxity, max_wires, gate_library),
        }
    }

    pub fn lookup_truth_table(&self, tt: &Vec<usize>) -> Option<usize> {
        self.table.get(tt).copied()
    }
}

fn build_complexity_table(
    max_cxity: usize,
    max_wires: usize,
    gate_library: GateLibrary,
) -> HashMap<Vec<usize>, usize> {
    assert!(max_cxity >= 1);
    assert!(max_wires >= 3);

    let tt_size = 1 << max_wires;

    let mut table = HashMap::new();
    // Identity gate
    table.insert((0..tt_size).collect(), 0);

    let mut current_layer: Vec<Vec<usize>> = vec![(0..tt_size).collect()];

    for cxity in 1..max_cxity {
        println!(
            "{} truth tables of new cxity. Processing {} candidates now",
            current_layer.len(),
            current_layer.len()
                * gate_library.cfs().len()
                * max_wires
                * (max_wires - 1)
                * (max_wires - 2)
                / 2
        );
        let mut next_layer = vec![];

        for t in 0..max_wires {
            for c1 in 0..max_wires {
                if t != c1 {
                    for c2 in c1 + 1..max_wires {
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
        max_cxity - 1,
        current_layer.len() * max_wires * (max_wires - 1) * (max_wires - 2) / 2,
        max_cxity,
    );

    // Max cxity
    for t in 0..max_wires {
        for c1 in 0..max_wires {
            if t != c1 {
                for c2 in c1 + 1..max_wires {
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
                                        e.insert(max_cxity);
                                        num_added += 1;
                                    }
                                    std::collections::hash_map::Entry::Occupied(_) => {}
                                }
                            }
                            println!(
                                "cxity = {}, num added = {}, gate = {:?}",
                                max_cxity, num_added, next_gate
                            );
                        }
                    }
                }
            }
        }
    }

    table
}

#[cfg(test)]
mod test {
    use std::time::Instant;

    use crate::{
        circuit::{
            analysis::{projection_circuit, truth_table},
            cf::GateLibrary,
            Circuit,
        },
        compression::cxity_table::PermutationComplexityTable,
    };

    #[test]
    fn test_build_complexity_table() {
        let gates = 3;
        let wires = 9;
        let gate_library = GateLibrary::TwoBit;
        let s = Instant::now();
        let ct = PermutationComplexityTable::new(gates, wires, gate_library);
        let d = Instant::now() - s;
        dbg!(d);

        let mut rng = rand::rng();
        for _ in 0..1000000 {
            let circuit =
                Circuit::random_with_cf(wires, gates, GateLibrary::TwoBit, &mut rng).gates;
            let (proj_circuit, _) = projection_circuit(&circuit);
            let tt = truth_table(wires, &proj_circuit);
            let res = ct.lookup_truth_table(&tt);
            if res.is_none() {
                dbg!(&circuit);
                dbg!(&proj_circuit);
            }
            assert!(res.is_some());
        }
    }
}
