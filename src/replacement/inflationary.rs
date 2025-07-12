use std::collections::HashMap;

use rand::{seq::IndexedRandom, Rng};
use rand::{RngCore, SeedableRng};
use rayon::current_num_threads;
use rayon::iter::{ParallelBridge, ParallelIterator};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::OnceLock;

use crate::circuit::{
    analysis::{projection_circuit, truth_table},
    cf::GateLibrary,
    circuit::{circuit_min_generation, correct_controls, evaluate_usize, to_string},
    Gate,
};

pub fn find_replacement_inf_common_target<R: Send + Sync + RngCore + SeedableRng>(
    input: &[Gate],
    replacement_size: usize,
    max_circuit_samples: usize,
    gate_library: GateLibrary,
    rng: &mut R,
) -> Option<(Vec<Gate>, usize)> {
    let proj_circuit = vec![
        Gate {
            wires: [0, 1, 2],
            control_func: input[0].control_func,
            generation: 0,
        },
        Gate {
            wires: [0, 3, 4],
            control_func: input[1].control_func,
            generation: 0,
        },
    ];
    let proj_map = vec![
        input[0].wires[0],
        input[0].wires[1],
        input[0].wires[2],
        input[1].wires[1],
        input[1].wires[2],
    ];

    let input_inout_table = truth_table(5, &proj_circuit);

    let num_threads = current_num_threads();
    let max_iterations = max_circuit_samples / num_threads;

    let found = AtomicBool::new(false);
    let replacement_res = Arc::new(OnceLock::new());
    let sample_count_res: usize = (0..num_threads)
        .map(|_| R::from_rng(rng))
        .par_bridge()
        .map(|mut rng| {
            let epoch_size = rng.random_range(10..20);
            let mut replacement_circuit;
            for iter in 1..=max_iterations {
                if iter % epoch_size == 0 && found.load(Ordering::Relaxed) {
                    return iter;
                }

                replacement_circuit =
                    sample_circuit_inf_common_target(replacement_size, gate_library, &mut rng);

                // functional equivalence
                let mut func_equiv = true;
                for i in 0..1 << 5 {
                    let mut input = i;
                    replacement_circuit
                        .iter()
                        .for_each(|g| input = g.evaluate_usize(input));
                    if input != input_inout_table[i] {
                        func_equiv = false;
                        break;
                    }
                }

                if !func_equiv {
                    continue;
                }

                found.store(true, Ordering::Relaxed);

                if replacement_res.set(replacement_circuit).is_ok() {
                    found.store(true, Ordering::Relaxed);
                    return iter;
                }
            }

            max_iterations
        })
        .sum();

    if let Some(replacement_circuit) = replacement_res.get() {
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
                        let orig_w = rng.random_range(0..5);
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

        // update gate generation
        let min_generation = input.iter().map(|g| g.generation).min().unwrap_or(0);
        let new_generation = min_generation + 1;
        output_circuit
            .iter_mut()
            .for_each(|g| g.generation = new_generation);

        return Some((output_circuit, sample_count_res));
    }

    None
}

fn sample_circuit_inf_common_target<R: Send + Sync + RngCore + SeedableRng>(
    replacement_size: usize,
    gate_library: GateLibrary,
    rng: &mut R,
) -> Vec<Gate> {
    // assumes 0 is common target, 1..4 are controls

    (0..replacement_size)
        .map(|_| {
            let c1 = rng.random_range(0..=3);
            let c2 = rng.random_range(c1 + 1..=4);
            let cf = gate_library.cfs().choose(rng).copied().unwrap();
            Gate {
                wires: [0, c1, c2],
                control_func: cf,
                generation: 0,
            }
        })
        .collect()
}

pub fn find_replacement_inflationary<R: Rng>(
    circuit: &[Gate],
    num_wires: usize,
    gate_library: GateLibrary,
    table: &HashMap<Vec<usize>, Vec<Vec<Gate>>>,
    max_circuit_samples: usize,
    rng: &mut R,
) -> Option<(Vec<Gate>, usize)> {
    let (proj_circuit, proj_map) = projection_circuit(circuit);

    const TT_SIZE: usize = 1 << 5;

    const VALID_BITLINES: [[usize; 3]; 30] = {
        let mut bitlines: [[usize; 3]; 30] = [[0, 0, 0]; 30];
        let mut i = 0;
        let mut t = 0;
        while t < 5 {
            let mut c1 = 0;
            while c1 < 5 {
                if t != c1 {
                    let mut c2 = c1 + 1;
                    while c2 < 5 {
                        if t != c2 {
                            bitlines[i] = [t, c1, c2];
                            i += 1;
                        }
                        c2 += 1;
                    }
                }
                c1 += 1;
            }
            t += 1;
        }

        bitlines
    };

    let mut num_samples = 0;
    let replacement_circuit = loop {
        if num_samples == max_circuit_samples {
            println!("Failed on \n{}", to_string(&proj_circuit));
            return None;
        }
        num_samples += 1;
        let mut lhs = vec![];
        for _ in 0..5 {
            lhs.push(Gate {
                wires: VALID_BITLINES.choose(rng).copied().unwrap(),
                control_func: gate_library.cfs().choose(rng).copied().unwrap(),
                generation: 0,
            });
        }

        let mut tt: Vec<usize> = (0..TT_SIZE).collect();
        tt.iter_mut().for_each(|x| {
            *x = evaluate_usize(&lhs, *x);
            *x = evaluate_usize(&proj_circuit, *x);
        });
        if let Some(rhs_candidates) = table.get(&tt) {
            let rhs = rhs_candidates.choose(rng).cloned().unwrap();
            lhs.reverse();
            lhs.extend(rhs);

            // TODO: ab -> b'Xa', where a.wires = a'.wires and b.wires = b'.wires
            if lhs[0].wires != proj_circuit[1].wires
                || lhs[lhs.len() - 1].wires != proj_circuit[0].wires
            {
                continue;
            }

            break lhs;
        }
    };

    // map back to original num_wires
    let mut output_circuit = replacement_circuit.clone();
    let mut proj_map_new_wires = vec![];
    output_circuit.iter_mut().for_each(|g| {
        g.wires.iter_mut().for_each(|w| {
            let w_usize = *w;
            if w_usize < proj_map.len() {
                *w = proj_map[w_usize];
            } else if let Some((_, orig_w)) = proj_map_new_wires.iter().find(|(ww, _)| w == ww) {
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

    // update gate generation
    let min_generation = circuit_min_generation(circuit);
    let new_generation = min_generation + 1;
    output_circuit
        .iter_mut()
        .for_each(|g| g.generation = new_generation);

    Some((output_circuit, num_samples))
}
