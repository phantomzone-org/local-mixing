pub mod inflationary;
pub mod replace_ct;

use crate::circuit::{
    analysis::{compute_active_wires, projection_circuit, truth_table},
    cf::GateLibrary,
    circuit::correct_controls,
    Gate,
};
use rand::{seq::IndexedRandom, Rng, RngCore, SeedableRng};
use rayon::{
    current_num_threads,
    iter::{ParallelBridge, ParallelIterator},
};
use std::sync::{
    atomic::{AtomicBool, Ordering::Relaxed},
    Arc, OnceLock,
};

#[inline]
pub fn is_weakly_connected(circuit: &[Gate]) -> bool {
    // weak-connectedness
    let mut visited = vec![false; circuit.len()];
    let mut stack = vec![0; circuit.len()];
    let mut stack_size = 1;
    visited[0] = true;

    while stack_size > 0 {
        stack_size -= 1;
        let current = stack[stack_size];
        for i in 0..circuit.len() {
            if !visited[i] && circuit[current].collides_with(&circuit[i]) {
                visited[i] = true;
                stack[stack_size] = i;
                stack_size += 1;
            }
        }
    }

    visited.iter().all(|&v| v)
}

pub fn find_replacement_random_sample<R: Send + Sync + RngCore + SeedableRng>(
    circuit: &[Gate],
    num_wires: usize,
    replacement_size: usize,
    max_circuit_samples: usize,
    gate_library: GateLibrary,
    output_connected: bool,
    rng: &mut R,
) -> Option<(Vec<Gate>, usize)> {
    let (proj_circuit, proj_map) = projection_circuit(circuit);
    let num_projection_wires = proj_map.len() + 2;
    let input_inout_table = truth_table(num_projection_wires, &proj_circuit);
    let tt_size = 1 << num_projection_wires;
    let (active_targets, active_controls) =
        compute_active_wires(proj_map.len(), &input_inout_table);

    let num_threads = current_num_threads();
    let max_iterations = max_circuit_samples / num_threads;

    let found = AtomicBool::new(false);
    let replacement_res = Arc::new(OnceLock::new());
    let sample_count_res = (0..num_threads)
        .map(|_| R::from_rng(rng))
        .par_bridge()
        .map(|mut rng| {
            let epoch_size = rng.random_range(10..20);
            let mut replacement_circuit;
            for iter in 1..=max_iterations {
                if iter % epoch_size == 0 && found.load(Relaxed) {
                    return iter;
                }

                replacement_circuit = sample_random_circuit(
                    replacement_size,
                    num_projection_wires,
                    &active_targets,
                    &active_controls,
                    gate_library,
                    &mut rng,
                );

                // functional equivalence
                let mut func_equiv = true;
                for i in 0..tt_size {
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

                if output_connected && !is_weakly_connected(&replacement_circuit) {
                    continue;
                }

                if replacement_res.set(replacement_circuit).is_ok() {
                    found.store(true, Relaxed);
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
        let min_generation = circuit.iter().map(|g| g.generation).min().unwrap_or(0);
        let new_generation = min_generation + 1;
        output_circuit
            .iter_mut()
            .for_each(|g| g.generation = new_generation);

        return Some((output_circuit, sample_count_res));
    }

    None
}

#[inline]
pub fn sample_random_circuit<R: Send + Sync + RngCore + SeedableRng>(
    replacement_size: usize,
    num_projection_wires: usize,
    active_targets: &Vec<usize>,
    active_controls: &Vec<usize>,
    gate_library: GateLibrary,
    rng: &mut R,
) -> Vec<Gate> {
    let mut circuit = vec![Gate::default(); replacement_size];
    let mut placed_wire_in_gate: [Vec<bool>; 3] =
        std::array::from_fn(|_| vec![false; replacement_size]);

    // Place active target wires
    for w in active_targets {
        loop {
            let gate_idx = rng.random_range(0..replacement_size);
            if !placed_wire_in_gate[0][gate_idx] {
                circuit[gate_idx].wires[0] = *w;
                placed_wire_in_gate[0][gate_idx] = true;
                break;
            }
        }
    }

    // Place active control wires
    'active_control: loop {
        for w in active_controls {
            // Probability that any slot (there are 2*N_IN) is not sampled in 3*N_IN iterations
            // is 1/( 2*N_IN )^{3*N_IN} which is very low.
            // For ex, when N_IN=4 non-sampling probability is 1/(8^12) = 1/2^{36}
            let mut placed = false;
            for _ in 0..3 * replacement_size {
                let index = rng.random_range(0..2 * replacement_size);
                let (gate_idx, control_idx) = (index >> 1, (index & 1) + 1);
                // Check if the same wire is acting as target (and is placed)
                if placed_wire_in_gate[0][gate_idx] && circuit[gate_idx].wires[0] == *w {
                    continue;
                }

                if !placed_wire_in_gate[control_idx][gate_idx] {
                    circuit[gate_idx].wires[control_idx] = *w;
                    placed_wire_in_gate[control_idx][gate_idx] = true;
                    placed = true;
                    break;
                }
            }

            // Placement is impossible with very high probability, try setting active control wires again
            if !placed {
                placed_wire_in_gate[1] = vec![false; replacement_size];
                placed_wire_in_gate[2] = vec![false; replacement_size];
                continue 'active_control;
            }
        }

        break;
    }

    for gate_idx in 0..replacement_size {
        let mut set = vec![false; num_projection_wires];
        for i in 0..3 {
            if placed_wire_in_gate[i][gate_idx] {
                set[circuit[gate_idx].wires[i]] = true;
            }
        }
        for i in 0..3 {
            if !placed_wire_in_gate[i][gate_idx] {
                circuit[gate_idx].wires[i] = loop {
                    let v = rng.random_range(0..num_projection_wires);
                    if !set[v] {
                        set[v] = true;
                        break v;
                    }
                };
            }
        }
        circuit[gate_idx].control_func = gate_library.cfs().choose(rng).copied().unwrap();
    }

    circuit
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use rand::{seq::IndexedRandom, SeedableRng};
    use rand_chacha::ChaCha8Rng;
    use serde::{Deserialize, Serialize};

    use crate::{
        circuit::{
            analysis::num_distinct_wires,
            cf::GateLibrary,
            circuit::{check_equiv_probabilistic, par_check_equiv_probabilistic, to_string},
            Circuit, Gate,
        },
        compression::ct::CompressionTable,
        replacement::{
            inflationary::find_replacement_inf_common_target, is_weakly_connected,
            replace_ct::find_replacement_with_ct,
        },
    };

    use super::find_replacement_random_sample;

    #[test]
    fn test_find_replacement_random_sample() {
        let wires = 15;
        let mut rng = ChaCha8Rng::from_os_rng();
        for _ in 0..10 {
            let ckt_one =
                Circuit::random_with_cf(wires, 2, GateLibrary::OnlyUnique, &mut rng).gates;
            let replacement = match find_replacement_random_sample(
                &ckt_one,
                wires,
                4,
                1_000_000_000,
                GateLibrary::OnlyUnique,
                true,
                &mut rng,
            ) {
                Some((r, _)) => r,
                None => panic!(),
            };
            match par_check_equiv_probabilistic(wires, &ckt_one, &replacement, 1000, &mut rng) {
                Ok(()) => continue,
                _ => {
                    dbg!(ckt_one);
                    dbg!(replacement);
                    panic!();
                }
            }
        }
    }

    #[test]
    fn test_replacement_properties() {
        // Script config
        let n_input_circuits = 10;
        let n_replacement_attempts_per_input = 10;
        let n_attempts_to_generate_input = 100000;

        let circuit_num_wires = 64;
        let gate_library = GateLibrary::TwoBit;

        // Input circuit config
        let input_num_gates = 2;
        let input_min_wires = 3;
        let input_max_wires = 6;
        let input_wires_save = Some(vec![[2, 0, 1], [2, 3, 4]]);
        let input_wc = false;

        // Output circuit config
        let output_num_gates = 4;
        let output_min_wires = 3;
        let output_max_wires = 5;
        let enforce_output_wc = false;

        // Replacement strategy config
        let random_sample_max_circuit_samples = 1000000;

        let ct_sample_max_circuit_samples = 20;
        let ct_max_gates = 3;
        let ct_max_wires = 9;
        let ct_save: Option<String> = Some(String::from("bin/table-twobit.db"));
        let mut ct = CompressionTable::empty();

        #[derive(PartialEq)]
        enum ReplacementFn {
            RandomSample,
            CTSample,
        }
        let replacement_fn = ReplacementFn::RandomSample;

        let mut rng = ChaCha8Rng::from_os_rng();

        let input_samples: Vec<Vec<Gate>>;
        match input_wires_save {
            Some(wires_template) => {
                println!("Generating according to template");
                input_samples = (0..n_input_circuits)
                    .map(|_| {
                        wires_template
                            .iter()
                            .map(|&wires| Gate {
                                wires,
                                control_func: gate_library.cfs().choose(&mut rng).copied().unwrap(),
                                generation: 0,
                            })
                            .collect::<Vec<Gate>>()
                    })
                    .collect();
            }
            None => {
                println!("Generating {} input circuits: gates = {}, {} <= wires <= {}, gate library = {:?}, wc = {}", n_input_circuits, input_num_gates, input_min_wires, input_max_wires, gate_library, input_wc);
                input_samples = (0..n_input_circuits)
                    .map(|_| {
                        for _ in 0..n_attempts_to_generate_input {
                            let circuit = Circuit::random_with_cf(
                                input_max_wires,
                                input_num_gates,
                                gate_library,
                                &mut rng,
                            )
                            .gates;

                            if num_distinct_wires(&circuit) < input_min_wires {
                                continue;
                            }

                            if input_wc && !is_weakly_connected(&circuit) {
                                continue;
                            }

                            return circuit;
                        }

                        panic!(
                            "Failed to sample input in time. n_attempts allowed: {}",
                            n_attempts_to_generate_input
                        );
                    })
                    .collect();
            }
        }

        if replacement_fn == ReplacementFn::CTSample {
            if let Some(path) = ct_save {
                ct = CompressionTable::from_file(&path);
                assert_eq!(ct.max_gates_supported, ct_max_gates);
                assert_eq!(ct.max_wires_supported, ct_max_wires);
                assert_eq!(ct.gate_library, gate_library);
            } else {
                ct = CompressionTable::new(ct_max_gates, ct_max_wires, gate_library);
            }
        }

        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
        enum ReplacementError {
            ReturnFail,
            NotFuncEquiv,
            TooFewWires,
            TooManyWires,
            FailedWeakConnectedCheck,
        }

        #[derive(Serialize, Deserialize)]
        struct ResultType {
            input: Vec<Gate>,
            outputs: Vec<Result<Vec<Gate>, ReplacementError>>,
        }

        println!("Replacement:");
        let _results: Vec<ResultType> = (0..n_input_circuits)
            .map(|i| {
                let mut results = vec![];
                for _ in 0..n_replacement_attempts_per_input {
                    let res = match replacement_fn {
                        ReplacementFn::CTSample => find_replacement_with_ct(
                            &input_samples[i],
                            circuit_num_wires,
                            output_num_gates,
                            ct_sample_max_circuit_samples,
                            &ct,
                            enforce_output_wc,
                            false,
                            &mut rng,
                        ),
                        ReplacementFn::RandomSample => find_replacement_inf_common_target(
                            &input_samples[i],
                            output_num_gates,
                            random_sample_max_circuit_samples,
                            gate_library,
                            &mut rng,
                        ),
                    };

                    let output = match res {
                        Some((out, _)) => out,
                        None => {
                            results.push(Err(ReplacementError::ReturnFail));
                            continue;
                        }
                    };

                    if check_equiv_probabilistic(circuit_num_wires, &input_samples[i], &output, 1000, &mut rng).is_err() {
                        results.push(Err(ReplacementError::NotFuncEquiv));
                        continue;
                    }

                    let wires = num_distinct_wires(&output);
                    if wires < output_min_wires {
                        results.push(Err(ReplacementError::TooFewWires));
                        continue;
                    } else if wires > output_max_wires {
                        results.push(Err(ReplacementError::TooManyWires));
                        continue;
                    } else if enforce_output_wc && !is_weakly_connected(&output) {
                        results.push(Err(ReplacementError::FailedWeakConnectedCheck));
                        continue;
                    }

                    results.push(Ok(output));
                }

                let mut num_success = 0;
                let mut num_success_wc = 0;
                let mut mean_num_output_wires = 0.0;
                let mut error_freq: HashMap<ReplacementError, i32> = HashMap::new();
                results.iter().for_each(|status| {
                    match status {
                        Ok(output) => {
                            num_success += 1;
                            if is_weakly_connected(&output) {
                                num_success_wc += 1;
                            }
                            mean_num_output_wires += num_distinct_wires(&output) as f64;
                        }
                        Err(e) => {
                            error_freq
                                .entry(*e)
                                .and_modify(|freq| *freq += 1)
                                .or_insert(1);
                        }
                    };
                });

                mean_num_output_wires /= num_success as f64;

                println!("Iteration {} summary:", i);
                println!(
                    "Input has {} gates, {} wires, is_wc = {}",
                    input_num_gates,
                    num_distinct_wires(&input_samples[i]),
                    input_wc
                );
                println!("input = \n{}", to_string(&input_samples[i]));
                println!(
                    "Replacements: {} failed, {} succeeded. Mean number of output wires = {}. {} successes are wc.",
                    n_replacement_attempts_per_input - num_success,
                    num_success,
                    mean_num_output_wires,
                    num_success_wc,
                );
                println!("Errors: {:?}", error_freq);
                let mut unique_outputs = std::collections::HashSet::new();
                for status in &results {
                    if let Ok(output) = status {
                        unique_outputs.insert(to_string(output));
                    }
                }
                println!("All unique successful outputs:");
                for output_str in &unique_outputs {
                    println!("{}\n", output_str);
                }

                ResultType {
                    input: input_samples[i].clone(),
                    outputs: results,
                }
            })
            .collect();
    }
}
