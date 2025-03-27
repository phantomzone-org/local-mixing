pub mod strategy;
pub mod test;

use crate::{
    circuit::{
        analysis::{compute_active_wires, projection_circuit, truth_table},
        Gate,
    },
    local_mixing::tracer::ReplacementTraceFields,
};
use rand::{Rng, RngCore, SeedableRng};
use rayon::{
    current_num_threads,
    iter::{ParallelBridge, ParallelIterator},
};
use std::{
    array::from_fn,
    iter::repeat_with,
    sync::atomic::{AtomicBool, Ordering::Relaxed},
};
use strategy::{ControlFnChoice, ReplacementStrategy};

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

pub fn find_replacement_circuit<
    const N_OUT: usize,
    const N_IN: usize,
    const N_PROJ_WIRES: usize,
    const N_PROJ_INPUTS: usize,
    R: Send + Sync + RngCore + SeedableRng,
>(
    circuit: &[Gate; N_OUT],
    num_wires: usize,
    num_attempts: usize,
    strategy: ReplacementStrategy,
    cf_choice: ControlFnChoice,
    rng: &mut R,
) -> Option<([Gate; N_IN], ReplacementTraceFields)> {
    let (proj_circuit, proj_map) = projection_circuit(&circuit.to_vec());
    let tt = truth_table(proj_map.len(), &proj_circuit);
    let active_wires_vecs = compute_active_wires(proj_map.len(), &tt);

    let mut num_active_wires = 0;
    let mut active_wires = [[false; N_PROJ_WIRES]; 2];
    active_wires_vecs.0.iter().for_each(|&w| {
        num_active_wires += 1;
        active_wires[0][w] = true;
    });
    active_wires_vecs.1.iter().for_each(|&w| {
        if !active_wires[1][w] && !active_wires[0][w] {
            num_active_wires += 1;
        }
        active_wires[1][w] = true;
    });

    if num_active_wires > N_PROJ_WIRES {
        println!(
            "num_active_wires > N_PROJ_WIRES: {} > {}",
            num_active_wires, N_PROJ_WIRES
        );
        return None;
    }

    let eval_table = truth_table(N_PROJ_WIRES, &proj_circuit);

    let mut input_distinct = vec![];
    proj_circuit.iter().for_each(|g| {
        g.wires.iter().for_each(|w| {
            if !input_distinct.contains(w) {
                input_distinct.push(*w);
            }
        })
    });

    let max_iterations = num_attempts / current_num_threads();
    let found = AtomicBool::new(false);

    let sample_function: Box<
        dyn Fn(&mut [Gate; N_IN], &[[bool; N_PROJ_WIRES]; 2], &mut R) + Send + Sync,
    > = match strategy {
        ReplacementStrategy::SampleUnguided => Box::new(|replacement_circuit, _, rng| {
            sample_random_circuit_unguided::<N_IN, N_PROJ_WIRES, _>(
                replacement_circuit,
                cf_choice,
                rng,
            );
        }),
        ReplacementStrategy::SampleActive0 => Box::new(|replacement_circuit, active_wires, rng| {
            sample_random_circuit(replacement_circuit, &active_wires, cf_choice, rng);
        }),
        _ => todo!(),
    };

    let res = (0..current_num_threads())
        .map(|_| R::from_rng(rng))
        .par_bridge()
        .find_map_any(|mut rng| {
            let epoch_size = rng.random_range(10..20);
            let mut replacement_circuit = [Gate::default(); N_IN];
            for iter in 1..=max_iterations {
                if iter % epoch_size == 0 && found.load(Relaxed) {
                    return None;
                }

                sample_function(&mut replacement_circuit, &active_wires, &mut rng);

                // functional equivalence
                let mut func_equiv = true;
                for i in 0..N_PROJ_INPUTS {
                    let mut input = i;
                    replacement_circuit.iter().for_each(|g| {
                        let a = (input & (1 << g.wires[1])) != 0;
                        let b = (input & (1 << g.wires[2])) != 0;
                        let x = g.evaluate_cf(a, b);
                        input ^= (x as usize) << g.wires[0];
                    });
                    if input != eval_table[i] {
                        func_equiv = false;
                        break;
                    }
                }

                if !func_equiv {
                    continue;
                }

                if !is_weakly_connected::<N_IN>(&replacement_circuit) {
                    continue;
                }

                found.store(true, Relaxed);
                return Some((replacement_circuit, iter));
            }

            None
        });

    if let Some((mut replacement_circuit, iter)) = res {
        let mut proj_map_new_wires = vec![];
        replacement_circuit.iter_mut().for_each(|g| {
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

        // update gate generation
        let min_generation = circuit.iter().map(|g| g.generation).min().unwrap_or(0);
        let new_generation = min_generation + 1;
        replacement_circuit
            .iter_mut()
            .for_each(|g| g.generation = new_generation);

        // output distinct wires
        let mut output_distinct = vec![];
        replacement_circuit.iter().for_each(|g| {
            g.wires.iter().for_each(|w| {
                if !output_distinct.contains(w) {
                    output_distinct.push(*w);
                }
            });
        });

        return Some((
            replacement_circuit,
            ReplacementTraceFields {
                num_input_wires: input_distinct.len(),
                num_output_wires: output_distinct.len(),
                num_active_wires,
                min_generation,
                num_circuits_sampled: iter,
            },
        ));
    }

    None
}

pub fn sample_random_circuit<
    const N_IN: usize,
    const N_PROJ_WIRES: usize,
    R: Send + Sync + RngCore + SeedableRng,
>(
    circuit: &mut [Gate; N_IN],
    active_wires: &[[bool; N_PROJ_WIRES]; 2],
    cf_choice: ControlFnChoice,
    rng: &mut R,
) {
    let mut placed_wire_in_gate = [[false; N_IN]; 3];

    // Place active target wires
    for i in 0..N_PROJ_WIRES {
        if !active_wires[0][i] {
            continue;
        }
        loop {
            let gate_idx = rng.random_range(0..N_IN);
            if !placed_wire_in_gate[0][gate_idx] {
                circuit[gate_idx].wires[0] = i;
                placed_wire_in_gate[0][gate_idx] = true;
                break;
            }
        }
    }

    // Place active control wires
    'active_control: loop {
        for w in 0..N_PROJ_WIRES {
            if !active_wires[1][w] {
                continue;
            }

            let mut placed = false;

            // Probability that any slot (there are 2*N_IN) is not sampled in 3*N_IN iterations
            // is 1/( 2*N_IN )^{3*N_IN} which is very low.
            // For ex, when N_IN=4 non-sampling probability is 1/(8^12) = 1/2^{36}
            for _ in 0..3 * N_IN {
                let index = rng.random_range(0..2 * N_IN);
                let (gate_idx, control_idx) = (index >> 1, (index & 1) + 1);
                // Check if the same wire is acting as target (and is placed)
                if placed_wire_in_gate[0][gate_idx] && circuit[gate_idx].wires[0] == w {
                    continue;
                }

                if !placed_wire_in_gate[control_idx][gate_idx] {
                    circuit[gate_idx].wires[control_idx] = w;
                    placed_wire_in_gate[control_idx][gate_idx] = true;
                    placed = true;
                    break;
                }
            }

            // Placement is impossible with very high probability, try setting active control wires again
            if !placed {
                placed_wire_in_gate[1] = [false; N_IN];
                placed_wire_in_gate[2] = [false; N_IN];
                continue 'active_control;
            }
        }

        break;
    }

    for gate_idx in 0..N_IN {
        let mut set: [bool; N_PROJ_WIRES] = [false; N_PROJ_WIRES];
        for i in 0..3 {
            if placed_wire_in_gate[i][gate_idx] {
                set[circuit[gate_idx].wires[i]] = true;
            }
        }
        for i in 0..3 {
            if !placed_wire_in_gate[i][gate_idx] {
                circuit[gate_idx].wires[i] = loop {
                    let v = rng.random_range(0..N_PROJ_WIRES);
                    if !set[v] {
                        set[v] = true;
                        break v;
                    }
                };
            }
        }
        circuit[gate_idx].control_func = cf_choice.random_cf(rng);
    }
}

pub fn sample_random_circuit_unguided<const N_IN: usize, const N_PROJ_WIRES: usize, R: Rng>(
    circuit: &mut [Gate; N_IN],
    cf_choice: ControlFnChoice,
    rng: &mut R,
) {
    let mut rng0 = repeat_with(|| rng.random_range(0..N_PROJ_WIRES));

    circuit.iter_mut().for_each(|gate| {
        let mut set: [bool; N_PROJ_WIRES] = [false; N_PROJ_WIRES];
        let [t, c0, c1] = from_fn(|_| loop {
            let v = rng0.next().unwrap();
            if !set[v] {
                set[v] = true;
                break v;
            }
        });

        gate.wires = [t, c0, c1];
    });

    circuit.iter_mut().for_each(|gate| {
        gate.control_func = cf_choice.random_cf(rng);
    });
}

#[cfg(test)]
mod tests {
    use rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;

    use crate::circuit::{circuit::check_equiv_probabilistic, Circuit};

    use super::{
        find_replacement_circuit,
        strategy::{ControlFnChoice, ReplacementStrategy},
    };

    #[test]
    fn test_find_replacement_n_out_4() {
        let wires = 100;
        let mut rng = ChaCha8Rng::from_os_rng();
        for _ in 0..10 {
            let ckt_one = Circuit::random(wires, 2, &mut rng);
            let replacement = match find_replacement_circuit::<2, 4, 9, { 1 << 9 }, _>(
                &[ckt_one.gates[0], ckt_one.gates[1]],
                wires,
                1_000_000_000,
                ReplacementStrategy::SampleActive0,
                ControlFnChoice::OnlyUnique,
                &mut rng,
            ) {
                Some((r, _)) => r,
                None => panic!(),
            };
            let ckt_two = Circuit {
                num_wires: wires,
                gates: Vec::from(replacement),
            };
            match check_equiv_probabilistic(
                wires,
                &ckt_one.gates,
                &Vec::from(replacement),
                1000,
                &mut rng,
            ) {
                Ok(()) => continue,
                _ => {
                    dbg!(ckt_one);
                    dbg!(ckt_two);
                }
            }
        }
    }
}
