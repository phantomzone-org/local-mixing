use std::collections::HashMap;

use ct::CompressionTable;
use rand::{
    seq::{IndexedRandom, SliceRandom},
    Rng,
};
use serde::{Deserialize, Serialize};

use crate::{
    circuit::{
        analysis::{inverse_truth_table, truth_table},
        cf::GateLibrary,
        circuit::{evaluate_usize, GateData},
        Gate,
    },
    local_mixing::search::is_convex,
};

pub mod compress;
pub mod ct;

fn compress_truth_table(tt: &Vec<usize>) -> Vec<u8> {
    bincode::serialize(tt).unwrap()
}

fn compress_circuit(ckt: &Vec<Gate>) -> Vec<u8> {
    let data: Vec<GateData> = ckt.iter().map(|&g| GateData::from(g)).collect();
    bincode::serialize(&data).unwrap()
}

fn uncompress_circuit(ckt_bytes: &Vec<u8>) -> Vec<Gate> {
    let data: Vec<GateData> = bincode::deserialize(&ckt_bytes).unwrap();
    data.iter().map(|&g| Gate::from(g)).collect()
}

#[derive(Serialize, Deserialize, PartialEq)]
pub struct IncompressibleCircuitsTable {
    num_wires: usize,
    circuits: Vec<Vec<u8>>,
    tt_to_circuits: HashMap<Vec<u8>, Vec<Vec<u8>>>,
}

impl IncompressibleCircuitsTable {
    pub fn generate(
        num_gates: usize,
        num_wires: usize,
        gate_library: GateLibrary,
        ct: &CompressionTable,
    ) -> Self {
        generate_incompressible_circuits(num_gates, num_wires, gate_library, ct)
    }

    pub fn from_file(path: &str) -> Self {
        let data = std::fs::read(path).expect("Failed to read file");
        bincode::deserialize(&data).expect("Failed to deserialize compression table")
    }
}

pub fn generate_incompressible_circuits(
    num_gates: usize,
    num_wires: usize,
    gate_library: GateLibrary,
    ct: &CompressionTable,
) -> IncompressibleCircuitsTable {
    let tt_size = 1 << num_wires;

    let mut opt_subcircuits: Vec<Vec<u8>> = vec![];
    let mut opt_tt_to_circuit_table: HashMap<Vec<u8>, Vec<Vec<u8>>> = HashMap::new();

    {
        let mut prev_opt_ckts: Vec<Vec<u8>> = vec![compress_circuit(&vec![])];
        let mut tt_cxity_table: HashMap<Vec<u8>, u8> = HashMap::new();
        tt_cxity_table.insert(compress_truth_table(&(0..tt_size).collect()), 0);

        for size in 1..num_gates {
            println!("Processing size {} circuits", size);
            let mut next_prev_opt_ckts: Vec<Vec<u8>> = vec![];
            for prev_bytes in &prev_opt_ckts {
                let prev = uncompress_circuit(prev_bytes);
                let prev_tt: Vec<_> = (0..tt_size).map(|x| evaluate_usize(&prev, x)).collect();
                for t in 0..num_wires {
                    for c1 in 0..num_wires {
                        if t != c1 {
                            for c2 in c1 + 1..num_wires {
                                if t != c2 {
                                    for cf in gate_library.cfs() {
                                        let g = Gate {
                                            wires: [t, c1, c2],
                                            control_func: cf,
                                            generation: 0,
                                        };
                                        let tt = compress_truth_table(
                                            &prev_tt.iter().map(|&x| g.evaluate_usize(x)).collect(),
                                        );
                                        if let Some(cxity) = tt_cxity_table.get(&tt) {
                                            if *cxity == size as u8 {
                                                let mut ckt = prev.clone();
                                                ckt.push(g);
                                                next_prev_opt_ckts.push(compress_circuit(&ckt));
                                            }
                                        } else {
                                            tt_cxity_table.insert(tt, size as u8);
                                            let mut ckt = prev.clone();
                                            ckt.push(g);
                                            next_prev_opt_ckts.push(compress_circuit(&ckt));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            prev_opt_ckts = next_prev_opt_ckts;
        }

        println!("Processing size max = {} circuits", num_gates);

        for prev_bytes in &prev_opt_ckts {
            let prev = uncompress_circuit(prev_bytes);
            let prev_tt: Vec<_> = (0..tt_size).map(|x| evaluate_usize(&prev, x)).collect();
            for t in 0..num_wires {
                for c1 in 0..num_wires {
                    if t != c1 {
                        for c2 in c1 + 1..num_wires {
                            if t != c2 {
                                for cf in gate_library.cfs() {
                                    let g = Gate {
                                        wires: [t, c1, c2],
                                        control_func: cf,
                                        generation: 0,
                                    };
                                    let tt = compress_truth_table(
                                        &prev_tt.iter().map(|&x| g.evaluate_usize(x)).collect(),
                                    );
                                    if !tt_cxity_table.contains_key(&tt) {
                                        let mut ckt = prev.clone();
                                        ckt.push(g);
                                        if let Some(cxity) = ct.lookup_truth_table(&truth_table(
                                            ct.max_wires_supported,
                                            &ckt,
                                        )) {
                                            if cxity == num_gates {
                                                let ckt_compressed = compress_circuit(&ckt);
                                                opt_subcircuits.push(ckt_compressed.clone());
                                                opt_tt_to_circuit_table
                                                    .entry(tt)
                                                    .or_insert_with(Vec::new)
                                                    .push(ckt_compressed);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    IncompressibleCircuitsTable {
        num_wires,
        circuits: opt_subcircuits,
        tt_to_circuits: opt_tt_to_circuit_table,
    }
}

pub fn random_identity_circuit<R: Rng>(
    incompressible_circuits_table: &IncompressibleCircuitsTable,
    ct: &CompressionTable,
    rng: &mut R,
) -> Option<Vec<Gate>> {
    let lhs = uncompress_circuit(&incompressible_circuits_table.circuits.choose(rng).unwrap());
    let tt_size = 1 << incompressible_circuits_table.num_wires;
    let tt = (0..tt_size).map(|x| evaluate_usize(&lhs, x)).collect();
    let inv_tt = inverse_truth_table(&tt);
    if let Some(rhs_candidates) = incompressible_circuits_table
        .tt_to_circuits
        .get(&compress_truth_table(&inv_tt))
    {
        let mut shuffled = (0..rhs_candidates.len()).collect::<Vec<_>>();
        shuffled.shuffle(rng);

        for i in shuffled {
            let mut ckt = lhs.clone();
            ckt.extend(uncompress_circuit(&rhs_candidates[i]));
            if accept_identity_circuit(incompressible_circuits_table.num_wires, &ckt, &ct) {
                return Some(ckt);
            }
        }

        return None;
    }

    return None;
}

fn accept_identity_circuit(num_wires: usize, ckt: &Vec<Gate>, ct: &CompressionTable) -> bool {
    for i in 0..ckt.len() {
        for j in i + 1..ckt.len() {
            for k in j + 1..ckt.len() {
                for l in k + 1..ckt.len() {
                    if is_convex(num_wires, ckt, &[i, j, k, l]) {
                        let tt = truth_table(
                            ct.max_wires_supported,
                            &vec![ckt[i], ckt[j], ckt[k], ckt[l]],
                        );
                        if let Some(cxity) = ct.lookup_truth_table(&tt) {
                            if cxity < 4 {
                                return false;
                            }
                        }
                    }
                }
            }
        }
    }
    true
}

pub fn inflate_gate<R: Rng>(
    g: &Gate,
    circuit_num_wires: usize,
    incompressible_circuits_table: &IncompressibleCircuitsTable,
    ct: &CompressionTable,
    rng: &mut R,
) -> Vec<Gate> {
    let g_proj = Gate {
        wires: [0, 1, 2],
        control_func: g.control_func,
        generation: 0,
    };

    let mut wire_mapping = vec![];
    wire_mapping.extend(g.wires);
    while wire_mapping.len() < incompressible_circuits_table.num_wires {
        let w = rng.random_range(0..circuit_num_wires);
        if !wire_mapping.contains(&w) {
            wire_mapping.push(w);
        }
    }

    loop {
        if let Some(id_ckt) = random_identity_circuit(incompressible_circuits_table, ct, rng) {
            if let Some(pos) = id_ckt.iter().position(|r| r.equal_to(&g_proj)) {
                let mut replacement: Vec<Gate> = vec![];
                replacement.extend(&id_ckt[pos + 1..]);
                replacement.extend(&id_ckt[..pos]);
                replacement.iter_mut().for_each(|r| {
                    r.wires[0] = wire_mapping[r.wires[0]];
                    r.wires[1] = wire_mapping[r.wires[1]];
                    r.wires[2] = wire_mapping[r.wires[2]];
                });

                return replacement;
            }
        }
    }
}

// Only works for incompressible 4-wire ckts
pub fn inflate_gate_to_block<R: Rng>(
    g: &Gate,
    circuit_num_wires: usize,
    incompressible_circuits_table: &IncompressibleCircuitsTable,
    ct: &CompressionTable,
    rng: &mut R,
) -> Vec<Gate> {
    let mut replacement = vec![*g];
    let mut available_wires: Vec<usize> = (0..circuit_num_wires)
        .filter(|w| !g.wires.contains(w))
        .collect();

    while available_wires.len() > 0 {
        let selected_idx = rng.random_range(0..replacement.len());
        let selected = replacement[selected_idx];
        let new_wire = available_wires.remove(rng.random_range(0..available_wires.len()));
        let wire_mapping = [
            selected.wires[0],
            selected.wires[1],
            selected.wires[2],
            new_wire,
        ];
        let selected_proj = Gate {
            wires: [0, 1, 2],
            control_func: selected.control_func,
            generation: 0,
        };

        loop {
            if let Some(id_ckt) = random_identity_circuit(incompressible_circuits_table, ct, rng) {
                if let Some(pos) = id_ckt.iter().position(|r| r.equal_to(&selected_proj)) {
                    if !id_ckt.iter().any(|g| g.wires.contains(&3)) {
                        continue;
                    }

                    let mut gate_replacement: Vec<Gate> = vec![];
                    gate_replacement.extend(&id_ckt[pos + 1..]);
                    gate_replacement.extend(&id_ckt[..pos]);
                    gate_replacement.iter_mut().for_each(|r| {
                        r.wires[0] = wire_mapping[r.wires[0]];
                        r.wires[1] = wire_mapping[r.wires[1]];
                        r.wires[2] = wire_mapping[r.wires[2]];
                    });

                    replacement.splice(selected_idx..=selected_idx, gate_replacement);
                    break;
                }
            }
        }
    }

    replacement
}

#[cfg(test)]
mod test {
    use std::{fs, time::Instant};

    use crate::{
        circuit::{
            cf::GateLibrary,
            circuit::{check_equiv_probabilistic, to_string, to_string_vertical},
            Gate,
        },
        compression::{
            ct::CompressionTable, generate_incompressible_circuits, inflate_gate_to_block,
            random_identity_circuit, IncompressibleCircuitsTable,
        },
    };

    use super::inflate_gate;

    #[test]
    fn test_get_identity_halves() {
        let ct = CompressionTable::from_file("bin/table-all-4-wires.db");
        let s = Instant::now();
        let incompressible_circuits_table =
            generate_incompressible_circuits(8, 3, GateLibrary::TwoBit, &ct);
        let d = Instant::now() - s;
        dbg!(d);

        let mut rng = rand::rng();
        let ct = CompressionTable::from_file("bin/table-twobit.db");
        if let Some(ckt) = random_identity_circuit(&incompressible_circuits_table, &ct, &mut rng) {
            let check = check_equiv_probabilistic(3, &ckt, &vec![], 10000, &mut rng);
            assert!(check.is_ok());
        } else {
            println!("no result");
        }
    }

    #[test]
    fn test_build_identity_circuit_components() {
        let num_gates = 4;
        let num_wires = 4;
        let gate_library = GateLibrary::TwoBit;
        let ct = CompressionTable::from_file("bin/table-all-4-wires.db");
        let path = "bin/4-gate-4-wire-TwoBit-optimal-halves.bin";
        let s = Instant::now();
        let res = generate_incompressible_circuits(num_gates, num_wires, gate_library, &ct);
        let d = Instant::now() - s;
        println!("Time to generate identity components: {:?}", d);
        let data = bincode::serialize(&res).expect("Failed to serialize compression table");
        fs::write(path, data).expect("Failed to write file");

        let s = Instant::now();
        let retrieve: IncompressibleCircuitsTable = bincode::deserialize_from(
            std::fs::File::open(path).expect("Failed to open bin/id_table.db"),
        )
        .expect("Failed to deserialize id_table");
        let d = Instant::now() - s;
        println!("time to load: {:?}", d);

        assert!(res == retrieve);
    }

    #[test]
    fn test_inflate_gate() {
        let ct = CompressionTable::from_file("bin/table-all-4-wires.db");
        let g = Gate {
            wires: [10, 15, 20],
            control_func: 8,
            generation: 0,
        };
        let incompressible_circuits_table: IncompressibleCircuitsTable = bincode::deserialize_from(
            std::fs::File::open("bin/4-gate-4-wire-TwoBit-optimal-halves.bin")
                .expect("Failed to open bin/id_table.db"),
        )
        .expect("Failed to deserialize id_table");
        let mut rng = rand::rng();
        let s = Instant::now();
        let res = inflate_gate(&g, 64, &incompressible_circuits_table, &ct, &mut rng);
        let d = Instant::now() - s;
        dbg!(d);
        println!("g:\n{}", to_string(&[g]));
        println!("res:\n{}", to_string(&res));
        assert!(check_equiv_probabilistic(64, &vec![g], &res, 10000, &mut rng).is_ok());
    }

    #[test]
    fn test_inflate_gate_to_block() {
        let ct = CompressionTable::from_file("bin/table-all-4-wires.db");
        let g = Gate {
            wires: [10, 4, 7],
            control_func: 8,
            generation: 0,
        };
        let incompressible_circuits_table: IncompressibleCircuitsTable = bincode::deserialize_from(
            std::fs::File::open("bin/4-gate-4-wire-TwoBit-optimal-halves.bin")
                .expect("Failed to open bin/id_table.db"),
        )
        .expect("Failed to deserialize id_table");
        let mut rng = rand::rng();
        println!("g:\n{}", to_string(&[g]));
        let s = Instant::now();
        let res = inflate_gate_to_block(&g, 16, &incompressible_circuits_table, &ct, &mut rng);
        let d = Instant::now() - s;
        dbg!(d);
        println!("res:\n{}", to_string_vertical(&res));
        assert!(check_equiv_probabilistic(16, &vec![g], &res, 10000, &mut rng).is_ok());
    }
}
