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
pub fn is_weakly_connected<const N: usize>(circuit: &[Gate]) -> bool {
    // weak-connectedness
    let mut visited = [false; N];
    let mut stack = [0; N];
    let mut stack_size = 1;
    visited[0] = true;

    while stack_size > 0 {
        stack_size -= 1;
        let current = stack[stack_size];
        for i in 0..N {
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
    use rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;

    use crate::circuit::{cf::GateLibrary, circuit::par_check_equiv_probabilistic, Circuit};

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
}
