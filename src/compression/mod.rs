use std::collections::HashMap;

use ct::CompressionTable;
use rand::{
    seq::{IndexedRandom, SliceRandom},
    Rng,
};

use crate::{
    circuit::{
        analysis::{inverse_truth_table, truth_table},
        cf::GateLibrary,
        circuit::evaluate_usize,
        Gate,
    },
    local_mixing::search::is_convex,
};

pub mod compress;
pub mod ct;

pub fn get_identity_halves(
    num_gates: usize,
    num_wires: usize,
    gate_library: GateLibrary,
) -> (Vec<Vec<Gate>>, HashMap<Vec<usize>, Vec<Vec<Gate>>>) {
    assert!(num_gates % 2 == 0);

    let tt_size = 1 << num_wires;

    let mut opt_subcircuits: Vec<Vec<Gate>> = vec![];
    let mut opt_tt_to_circuit_table: HashMap<Vec<usize>, Vec<Vec<Gate>>> = HashMap::new();

    {
        let mut prev_opt_ckts = vec![vec![]];
        let mut tt_cxity_table: HashMap<Vec<usize>, usize> = HashMap::new();
        tt_cxity_table.insert((0..tt_size).collect(), 0);

        for size in 1..num_gates / 2 {
            println!("Processing size {} circuits", size);
            let mut next_prev_opt_ckts = vec![];
            for prev in &prev_opt_ckts {
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
                                        let tt: Vec<_> =
                                            prev_tt.iter().map(|&x| g.evaluate_usize(x)).collect();
                                        if let Some(cxity) = tt_cxity_table.get(&tt) {
                                            if *cxity == size {
                                                let mut ckt = prev.clone();
                                                ckt.push(g);
                                                next_prev_opt_ckts.push(ckt);
                                            }
                                        } else {
                                            tt_cxity_table.insert(tt, size);
                                            let mut ckt = prev.clone();
                                            ckt.push(g);
                                            next_prev_opt_ckts.push(ckt);
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

        println!("Processing size max = {} circuits", num_gates / 2);

        for prev in &prev_opt_ckts {
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
                                    let tt: Vec<_> =
                                        prev_tt.iter().map(|&x| g.evaluate_usize(x)).collect();
                                    if !tt_cxity_table.contains_key(&tt) {
                                        let mut ckt = prev.clone();
                                        ckt.push(g);
                                        opt_subcircuits.push(ckt.clone());
                                        opt_tt_to_circuit_table
                                            .entry(tt)
                                            .or_insert_with(Vec::new)
                                            .push(ckt);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    (opt_subcircuits, opt_tt_to_circuit_table)
}

pub fn random_identity_circuit<R: Rng>(
    identity_halves: &Vec<Vec<Gate>>,
    num_half_wires: usize,
    tt_to_halves: &HashMap<Vec<usize>, Vec<Vec<Gate>>>,
    ct: &CompressionTable,
    rng: &mut R,
) -> Option<Vec<Gate>> {
    let lhs = identity_halves.choose(rng).unwrap();
    let tt_size = 1 << num_half_wires;
    let tt = (0..tt_size).map(|x| evaluate_usize(&lhs, x)).collect();
    let inv_tt = inverse_truth_table(&tt);
    if let Some(rhs_candidates) = tt_to_halves.get(&inv_tt) {
        let mut shuffled = (0..rhs_candidates.len()).collect::<Vec<_>>();
        shuffled.shuffle(rng);

        for i in shuffled {
            let mut ckt = lhs.clone();
            ckt.extend(rhs_candidates[i].clone());
            if accept_identity_circuit(num_half_wires, &ckt, &ct) {
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
    identity_halves: &Vec<Vec<Gate>>,
    num_half_wires: usize,
    tt_to_halves: &HashMap<Vec<usize>, Vec<Vec<Gate>>>,
    ct: &CompressionTable,
    rng: &mut R,
) -> Vec<Gate> {
    let g_proj = Gate {
        wires: [0, 1, 2],
        control_func: g.control_func,
        generation: 0,
    };

    loop {
        if let Some(id_ckt) =
            random_identity_circuit(identity_halves, num_half_wires, tt_to_halves, ct, rng)
        {
            if let Some(pos) = id_ckt.iter().position(|r| r.equal_to(&g_proj)) {
                let mut replacement: Vec<Gate> = vec![];
                replacement.extend(&id_ckt[pos + 1..]);
                replacement.extend(&id_ckt[..pos]);
                replacement.iter_mut().for_each(|r| {
                    r.wires[0] = g.wires[r.wires[0]];
                    r.wires[1] = g.wires[r.wires[1]];
                    r.wires[2] = g.wires[r.wires[2]];
                });

                return replacement;
            }
        }
    }
}

#[cfg(test)]
mod test {
    use std::{collections::HashMap, fs, time::Instant};

    use crate::{
        circuit::{
            cf::GateLibrary,
            circuit::{check_equiv_probabilistic, to_string},
            Gate,
        },
        compression::{ct::CompressionTable, random_identity_circuit},
    };

    use super::{get_identity_halves, inflate_gate};

    #[test]
    fn test_get_identity_halves() {
        let s = Instant::now();
        let (halves, tt_to_halves) = get_identity_halves(8, 3, GateLibrary::TwoBit);
        let d = Instant::now() - s;
        dbg!(d);

        let mut rng = rand::rng();
        let ct = CompressionTable::from_file("bin/table-twobit.db");
        if let Some(ckt) = random_identity_circuit(&halves, 3, &tt_to_halves, &ct, &mut rng) {
            let check = check_equiv_probabilistic(3, &ckt, &vec![], 10000, &mut rng);
            assert!(check.is_ok());
        } else {
            println!("no result");
        }
    }

    #[test]
    fn test_inflate_gate() {
        let ct = CompressionTable::from_file("bin/table-twobit.db");
        let (halves, tt_to_halves) = get_identity_halves(8, 3, GateLibrary::TwoBit);
        let g = Gate {
            wires: [10, 15, 20],
            control_func: 8,
            generation: 0,
        };
        let mut rng = rand::rng();
        let s = Instant::now();
        let res = inflate_gate(&g, &halves, 3, &tt_to_halves, &ct, &mut rng);
        let d = Instant::now() - s;
        dbg!(d);
        println!("g:\n{}", to_string(&[g]));
        println!("res:\n{}", to_string(&res));
        assert!(check_equiv_probabilistic(64, &vec![g], &res, 10000, &mut rng).is_ok());
    }

    #[test]
    fn test_build_identity_circuit_components() {
        let num_gates = 8;
        let num_wires = 3;
        let gate_library = GateLibrary::TwoBit;
        let path = "bin/4-gate-3-wire-TwoBit-optimal-halves.bin";
        let res = get_identity_halves(num_gates, num_wires, gate_library);
        let data = bincode::serialize(&res).expect("Failed to serialize compression table");
        fs::write(path, data).expect("Failed to write file");

        let s = Instant::now();
        let retrieve: (Vec<Vec<Gate>>, HashMap<Vec<usize>, Vec<Vec<Gate>>>) =
            bincode::deserialize_from(
                std::fs::File::open(path).expect("Failed to open bin/id_table.db"),
            )
            .expect("Failed to deserialize id_table");
        let d = Instant::now() - s;
        println!("time to load: {:?}", d);

        assert!(res == retrieve);
    }
}
