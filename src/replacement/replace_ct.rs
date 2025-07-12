use crate::circuit::analysis::{num_distinct_wires, projection_circuit, truth_table};
use crate::circuit::cf::GateLibrary;
use crate::circuit::circuit::{circuit_min_generation, correct_controls, evaluate_usize};
use crate::circuit::Gate;
use crate::compression::ct::CompressionTable;
use crate::local_mixing::classify_replacements::is_identity_subcircuits_replacement;
use rand::seq::IndexedRandom;
use rand::Rng;

use super::is_weakly_connected;

pub fn find_replacement_with_ct<R: Rng>(
    circuit: &[Gate],
    num_wires: usize,
    replacement_size: usize,
    max_circuit_samples: usize,
    ct: &CompressionTable,
    output_connected: bool,
    strictly_more_wires: bool,
    rng: &mut R,
) -> Option<(Vec<Gate>, usize)> {
    let (proj_circuit, proj_map) = projection_circuit(circuit);
    if proj_map.len() > ct.max_wires_supported {
        return None;
    }

    let proj_tt = truth_table(ct.max_wires_supported, &proj_circuit);
    let mut replacement_circuit = Vec::with_capacity(replacement_size);
    let mut num_samples = 0;

    let initial_gate_sample_size = if replacement_size > ct.max_gates_supported + 1 {
        replacement_size - ct.max_gates_supported
    } else {
        0
    };

    'sample_circuit: loop {
        num_samples += 1;
        if num_samples > max_circuit_samples {
            return None;
        }
        let mut curr_num_wires_used = proj_map.len();
        let mut lhs_tt = proj_tt.clone();
        let mut remaining_gates = replacement_size;
        replacement_circuit.clear();

        if initial_gate_sample_size > 0 {
            loop {
                let mut new_curr_num_wires_used = curr_num_wires_used;
                let mut initial_gates = Vec::with_capacity(initial_gate_sample_size);
                for _ in 0..initial_gate_sample_size {
                    let (gate, num_wires) = sample_next_projection_gate(
                        new_curr_num_wires_used,
                        ct.max_wires_supported,
                        ct.gate_library,
                        rng,
                    );
                    new_curr_num_wires_used = num_wires;
                    initial_gates.push(gate);
                }
                let new_lhs_tt: Vec<_> = lhs_tt
                    .iter()
                    .map(|&x| evaluate_usize(&initial_gates, x))
                    .collect();
                if let Some(rhs_cxity) = ct.lookup_truth_table(&new_lhs_tt) {
                    if rhs_cxity < remaining_gates {
                        lhs_tt = new_lhs_tt;
                        curr_num_wires_used = new_curr_num_wires_used;
                        replacement_circuit.extend(initial_gates);
                        remaining_gates -= initial_gate_sample_size;
                        break;
                    }
                }
            }
        }

        while remaining_gates > 0 {
            let (g, new_curr_num_wires_used) = sample_next_projection_gate(
                curr_num_wires_used,
                ct.max_wires_supported,
                ct.gate_library,
                rng,
            );
            let new_lhs_tt = lhs_tt.iter().map(|&x| g.evaluate_usize(x)).collect();
            if let Some(rhs_cxity) = ct.lookup_truth_table(&new_lhs_tt) {
                if rhs_cxity < remaining_gates {
                    if remaining_gates == 2 && rhs_cxity == 0 {
                        // 1 gate left to sample but rhs_cxity = 0
                        continue 'sample_circuit;
                    }
                    lhs_tt = new_lhs_tt;
                    curr_num_wires_used = new_curr_num_wires_used;
                    replacement_circuit.push(g);
                    remaining_gates -= 1;
                }
            }
        }
        replacement_circuit.reverse();

        // map back to original num_wires
        let mut output_circuit = replacement_circuit.clone();
        let mut proj_map_new_wires = vec![];
        output_circuit.iter_mut().for_each(|g| {
            g.wires.iter_mut().for_each(|w| {
                let w_usize = *w;
                if w_usize < proj_map.len() {
                    *w = proj_map[w_usize];
                } else if let Some((_, orig_w)) = proj_map_new_wires.iter().find(|(ww, _)| w == ww)
                {
                    *w = *orig_w;
                } else {
                    loop {
                        let orig_w = rng.random_range(0..num_wires);
                        if !proj_map.contains(&orig_w)
                            && !proj_map_new_wires.iter().any(|(_, ww)| *ww == orig_w)
                        {
                            proj_map_new_wires.push((w.clone(), orig_w));
                            *w = orig_w;
                            break;
                        }
                    }
                }
            });
        });

        correct_controls(&mut output_circuit);

        if output_connected && !is_weakly_connected(&output_circuit) {
            continue 'sample_circuit;
        }

        if strictly_more_wires
            && num_distinct_wires(&output_circuit) <= num_distinct_wires(&circuit)
        {
            continue 'sample_circuit;
        }

        if output_circuit.len() == circuit.len()
            && output_circuit.iter().all(|gate| {
                circuit
                    .iter()
                    .any(|g| g.wires == gate.wires && g.control_func == gate.control_func)
            })
        {
            continue 'sample_circuit;
        }

        if is_identity_subcircuits_replacement(&circuit, &output_circuit) {
            continue 'sample_circuit;
        }

        // update gate generation
        let min_generation = circuit_min_generation(circuit);
        let new_generation = min_generation + 1;
        output_circuit
            .iter_mut()
            .for_each(|g| g.generation = new_generation);

        return Some((output_circuit, num_samples));
    }
}

#[inline]
fn sample_next_projection_gate<R: Rng>(
    curr_num_wires_used: usize,
    max_num_wires_supported: usize,
    gate_library: GateLibrary,
    rng: &mut R,
) -> (Gate, usize) {
    loop {
        let mut target = rng.random_range(0..max_num_wires_supported);
        let mut control_one = rng.random_range(0..max_num_wires_supported);
        let mut control_two = rng.random_range(0..max_num_wires_supported);
        let mut new_num_wires_used = curr_num_wires_used;

        if target != control_one && target != control_two && control_one != control_two {
            if target >= curr_num_wires_used {
                target = new_num_wires_used;
                new_num_wires_used += 1;
            }
            if control_one >= curr_num_wires_used {
                control_one = new_num_wires_used;
                new_num_wires_used += 1;
            }
            if control_two >= curr_num_wires_used {
                control_two = new_num_wires_used;
                new_num_wires_used += 1;
            }
            return (
                Gate {
                    wires: [target, control_one, control_two],
                    control_func: gate_library.cfs().choose(rng).copied().unwrap(),
                    generation: 0,
                },
                new_num_wires_used,
            );
        }
    }
}

#[cfg(test)]
mod test {
    use std::time::Instant;

    use rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;

    use crate::{
        circuit::{cf::GateLibrary, circuit::check_equiv_probabilistic, Circuit},
        compression::ct::CompressionTable,
    };

    use super::{find_replacement_with_ct, sample_next_projection_gate};

    #[test]
    fn test_replacement_ct() {
        let ct = CompressionTable::from_file("bin/table-twobit.db");
        let wires = 10;
        let gates = 5;
        let mut rng = ChaCha8Rng::from_os_rng();

        for i in 1..=1 {
            let ckt_one =
                Circuit::random_with_cf(wires, gates, GateLibrary::TwoBit, &mut rng).gates;
            let s = Instant::now();
            match find_replacement_with_ct(&ckt_one, wires, gates, 100, &ct, false, false, &mut rng)
            {
                Some((r, samples)) => {
                    let d = Instant::now() - s;
                    println!("Iteration {}: SUCCESS. Time = {:?}", i, d);
                    println!("Input: {:?}", &ckt_one);
                    println!("Output: {:?}", &r);
                    println!("Samples: {}", samples);
                    assert!(check_equiv_probabilistic(wires, &ckt_one, &r, 1000, &mut rng).is_ok());
                }
                None => {
                    let d = Instant::now() - s;
                    println!("Iteration {}: FAIL. Time = {:?}", i, d);
                    println!("Input: {:?}", &ckt_one);
                }
            }
        }
    }

    #[test]
    fn test_sample_next_projection_gate() {
        let mut rng = rand::rng();
        let g = sample_next_projection_gate(5, 9, GateLibrary::TwoBit, &mut rng);
        dbg!(g);
    }
}
