pub mod strategy;
pub mod test;

use crate::{
    circuit::Gate,
    local_mixing::consts::{N_IN, N_PROJ_INPUTS, N_PROJ_WIRES, PROJ_GATE_CANDIDATES},
};
use rand::{
    seq::{IndexedRandom, SliceRandom},
    Rng, RngCore, SeedableRng,
};
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

pub fn find_replacement_circuit<R: Send + Sync + RngCore + SeedableRng, const N_OUT: usize>(
    circuit: &[Gate; N_OUT],
    num_wires: usize,
    num_attempts: usize,
    strategy: ReplacementStrategy,
    cf_choice: ControlFnChoice,
    rng: &mut R,
) -> Option<([Gate; N_IN], usize)> {
    let mut proj_circuit = [Gate::default(); N_OUT];
    let mut proj_map = [None; N_PROJ_WIRES];
    let mut proj_ctr = 0;
    let mut wire_already_placed;
    for i in 0..N_OUT {
        for w in 0..3 {
            wire_already_placed = false;
            for j in 0u32..proj_ctr {
                if proj_map[j as usize] == Some(circuit[i].wires[w]) {
                    wire_already_placed = true;
                    proj_circuit[i].wires[w] = j;
                    break;
                }
            }
            if !wire_already_placed {
                proj_map[proj_ctr as usize] = Some(circuit[i].wires[w]);
                proj_circuit[i].wires[w] = proj_ctr;
                proj_ctr += 1;
            }
        }
        proj_circuit[i].control_func = circuit[i].control_func;
    }

    let mut eval_table = [0; N_PROJ_INPUTS];
    for i in 0..N_PROJ_INPUTS {
        let mut input = i;
        proj_circuit.iter().for_each(|g| {
            let a = (input & (1 << g.wires[1])) != 0;
            let b = (input & (1 << g.wires[2])) != 0;
            let x = g.evaluate_cf(a, b);
            input ^= (x as usize) << g.wires[0];
        });
        eval_table[i as usize] = input;
    }

    let mut active_wires = [[false; N_PROJ_WIRES]; 2];
    for i in 0..N_PROJ_INPUTS {
        let eval_i = eval_table[i as usize];
        for gate_idx in 0..N_OUT {
            let t = proj_circuit[gate_idx].wires[0];
            if (eval_i & (1 << t)) != (i & (1 << t)) {
                active_wires[0][t as usize] = true;
            }
        }
    }

    for i in 0..N_PROJ_INPUTS {
        let eval_i = eval_table[i as usize];
        for gate_idx in 0..N_OUT {
            for control_idx in 1..=2 {
                let c = proj_circuit[gate_idx].wires[control_idx];
                let bit_mask = 1 << c;
                let i_flipped = i ^ bit_mask;
                let eval_i_flipped = eval_table[i_flipped as usize];
                let disagreement_bits = (eval_i ^ eval_i_flipped) & !bit_mask;
                if disagreement_bits != 0 {
                    active_wires[1][c as usize] = true;
                }
            }
        }
    }

    let max_iterations = num_attempts / current_num_threads();
    let found = AtomicBool::new(false);

    let sample_function: Box<
        dyn Fn(&mut [Gate; N_IN], &[[bool; N_PROJ_WIRES]; 2], &mut R) + Send + Sync,
    > = match strategy {
        ReplacementStrategy::SampleUnguided => Box::new(|replacement_circuit, _, rng| {
            sample_random_circuit_unguided(replacement_circuit, cf_choice, rng);
        }),
        ReplacementStrategy::SampleActive0 => Box::new(|replacement_circuit, active_wires, rng| {
            sample_random_circuit(replacement_circuit, &active_wires, cf_choice, rng);
        }),
        ReplacementStrategy::SampleActive1 => Box::new(|replacement_circuit, active_wires, rng| {
            sample_circuit_lookup(replacement_circuit, &active_wires, cf_choice, rng);
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
                    if input != eval_table[i as usize] {
                        func_equiv = false;
                        break;
                    }
                }

                if !func_equiv {
                    continue;
                }

                // weak-connectedness
                let mut visited = [false; N_IN];
                let mut stack = [0; N_IN];
                let mut stack_size = 1;
                visited[0] = true;

                while stack_size > 0 {
                    stack_size -= 1;
                    let current = stack[stack_size];
                    for i in 0..N_IN {
                        if !visited[i]
                            && replacement_circuit[current].collides_with(&replacement_circuit[i])
                        {
                            visited[i] = true;
                            stack[stack_size] = i;
                            stack_size += 1;
                        }
                    }
                }

                if !visited.iter().all(|&v| v) {
                    continue;
                }

                found.store(true, Relaxed);
                return Some((replacement_circuit, iter));
            }

            None
        });

    if let Some((mut replacement_circuit, iter)) = res {
        // replacement_circuit is accepted, map back to original space
        replacement_circuit.iter_mut().for_each(|g| {
            g.wires
                .iter_mut()
                .for_each(|w| match proj_map[*w as usize] {
                    Some(orig_w) => {
                        *w = orig_w;
                    }
                    None => loop {
                        let orig_w = rng.random_range(0..num_wires);
                        let some_orig_w = Some(orig_w as u32);
                        if !proj_map.contains(&some_orig_w) {
                            proj_map[*w as usize] = some_orig_w;
                            *w = orig_w as u32;
                            break;
                        }
                    },
                })
        });

        return Some((replacement_circuit, iter));
    }

    None
}

pub fn sample_random_circuit<R: Send + Sync + RngCore + SeedableRng>(
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
                circuit[gate_idx].wires[0] = i as u32;
                placed_wire_in_gate[0][gate_idx] = true;
                break;
            }
        }
    }

    // Place active control wires
    let mut all_slots: [_; 2 * N_IN] = std::array::from_fn(|i| (i / 2, 1 + (i % 2)));
    'active_control: loop {
        for w in 0..N_PROJ_WIRES {
            if !active_wires[1][w] {
                continue;
            }

            let mut placed = false;
            all_slots.shuffle(rng);
            for (gate_idx, control_idx) in all_slots {
                // Check if the same wire is acting as target (and is placed)
                if placed_wire_in_gate[0][gate_idx] && circuit[gate_idx].wires[0] == w as u32 {
                    continue;
                }

                if !placed_wire_in_gate[control_idx][gate_idx] {
                    circuit[gate_idx].wires[control_idx] = w as u32;
                    placed_wire_in_gate[control_idx][gate_idx] = true;
                    placed = true;
                    break;
                }
            }

            // Placement is impossible, try setting active control wires again
            if !placed {
                placed_wire_in_gate[1] = [false; N_IN];
                placed_wire_in_gate[2] = [false; N_IN];
                continue 'active_control;
            }
        }

        break;
    }

    for gate_idx in 0..N_IN {
        loop {
            let t;
            let c1;
            let c2;
            if placed_wire_in_gate[0][gate_idx] {
                t = circuit[gate_idx].wires[0];
            } else {
                t = rng.random_range(0..N_PROJ_WIRES) as u32;
            }
            if placed_wire_in_gate[1][gate_idx] {
                c1 = circuit[gate_idx].wires[1];
            } else {
                c1 = rng.random_range(0..N_PROJ_WIRES) as u32;
            }
            if placed_wire_in_gate[2][gate_idx] {
                c2 = circuit[gate_idx].wires[2];
            } else {
                c2 = rng.random_range(0..N_PROJ_WIRES) as u32;
            }
            if t != c1 && t != c2 && c1 != c2 {
                circuit[gate_idx].wires = [t, c1, c2];
                circuit[gate_idx].control_func = cf_choice.random_cf(rng);
                break;
            }
        }
    }
}

pub fn sample_random_circuit_unguided<R: Rng>(
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

        gate.wires = [t as u32, c0 as u32, c1 as u32];
    });

    circuit.iter_mut().for_each(|gate| {
        gate.control_func = cf_choice.random_cf(rng);
    });
}

pub fn sample_circuit_lookup<R: Rng>(
    circuit: &mut [Gate; N_IN],
    active_wires: &[[bool; N_PROJ_WIRES]; 2],
    cf_choice: ControlFnChoice,
    rng: &mut R,
) {
    let mut success = false;
    while !success {
        let mut circuit_contains_wire = [[false; N_PROJ_WIRES]; 2];
        circuit.iter_mut().for_each(|g| {
            g.wires = *PROJ_GATE_CANDIDATES.choose(rng).unwrap();

            circuit_contains_wire[0][g.wires[0] as usize] = true;
            circuit_contains_wire[1][g.wires[1] as usize] = true;
            circuit_contains_wire[1][g.wires[2] as usize] = true;
        });
        success = true;
        for w in 0..N_PROJ_WIRES {
            if active_wires[0][w] && !circuit_contains_wire[0][w] {
                success = false;
                break;
            }
            if active_wires[1][w] && !circuit_contains_wire[1][w] {
                success = false;
                break;
            }
        }

        circuit
            .iter_mut()
            .for_each(|g| g.control_func = cf_choice.random_cf(rng));
    }
}

#[cfg(test)]
mod tests {
    use rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;

    use crate::circuit::{circuit::is_func_equiv, Circuit};

    use super::{
        find_replacement_circuit,
        strategy::{ControlFnChoice, ReplacementStrategy},
    };

    #[test]
    fn test_find_replacement() {
        let wires = 100u32;
        let mut rng = ChaCha8Rng::from_os_rng();
        for _ in 0..1000 {
            let ckt_one = Circuit::random(wires, 2, &mut rng);
            let replacement = match find_replacement_circuit(
                &[ckt_one.gates[0], ckt_one.gates[1]],
                wires as usize,
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
            match is_func_equiv(&ckt_one, &ckt_two, 1000, &mut rng) {
                Ok(()) => continue,
                _ => {
                    dbg!(ckt_one);
                    dbg!(ckt_two);
                }
            }
        }
    }
}
