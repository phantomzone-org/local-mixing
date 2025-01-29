use std::{
    array::from_fn,
    iter::repeat_with,
    sync::atomic::{AtomicBool, Ordering::Relaxed},
};

use rand::{Rng, RngCore, SeedableRng};
use rayon::{
    current_num_threads,
    iter::{ParallelBridge, ParallelIterator},
};

use crate::circuit::{Base2GateControlFunc, Gate};

const RPL_GUIDED: bool = true;

pub fn find_replacement_circuit<
    R: Send + Sync + RngCore + SeedableRng,
    const N_OUT: usize,
    const N_IN: usize,
    const N_PROJ_WIRES: usize,
    const N_PROJ_INPUTS: usize,
>(
    circuit: &[Gate; N_OUT],
    num_wires: usize,
    num_attempts: usize,
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
            let x = Base2GateControlFunc::from_u8(g.control_func).evaluate(a, b);
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

    let res = (0..current_num_threads())
        .map(|_| R::from_rng(&mut *rng).unwrap())
        .par_bridge()
        .find_map_any(|mut rng| {
            let epoch_size = rng.gen_range(10..20);
            let mut replacement_circuit;
            // let mut placed_wire_in_gate;
            for iter in 1..=max_iterations {
                if iter % epoch_size == 0 && found.load(Relaxed) {
                    return None;
                }

                replacement_circuit = [Gate::default(); N_IN];
                // placed_wire_in_gate = [[false; 3]; N_IN];

                if RPL_GUIDED {
                    sample_random_circuit(
                        &mut replacement_circuit,
                        &active_wires,
                        // &mut placed_wire_in_gate,
                        &mut rng,
                    );
                } else {
                    sample_random_circuit_unguided::<_, N_IN, N_PROJ_WIRES>(
                        &mut replacement_circuit,
                        &mut rng,
                    );
                }

                // functional equivalence
                let mut func_equiv = true;
                for i in 0..N_PROJ_INPUTS {
                    let mut input = i;
                    replacement_circuit.iter().for_each(|g| {
                        let a = (input & (1 << g.wires[1])) != 0;
                        let b = (input & (1 << g.wires[2])) != 0;
                        let x = Base2GateControlFunc::from_u8(g.control_func).evaluate(a, b);
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
                        let orig_w = rng.gen_range(0..num_wires);
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

fn sample_random_circuit<
    R: Send + Sync + RngCore + SeedableRng,
    const N_IN: usize,
    const N_PROJ_WIRES: usize,
>(
    circuit: &mut [Gate; N_IN],
    active_wires: &[[bool; N_PROJ_WIRES]; 2],
    // placed_wire_in_gate: &mut [[bool; 3]; N_IN],
    rng: &mut R,
) {
    let mut placed_wire_in_gate = [[false; 3]; N_IN];

    for i in 0..N_PROJ_WIRES {
        if !active_wires[0][i] {
            continue;
        }
        loop {
            let gate_idx = rng.gen_range(0..N_IN);
            if !placed_wire_in_gate[gate_idx][0] {
                circuit[gate_idx].wires[0] = i as u32;
                placed_wire_in_gate[gate_idx][0] = true;
                break;
            }
        }
    }
    for i in 0..N_PROJ_WIRES {
        if !active_wires[1][i] {
            continue;
        }
        loop {
            let gate_idx = rng.gen_range(0..N_IN);
            if placed_wire_in_gate[gate_idx][0] && circuit[gate_idx].wires[0] == i as u32 {
                continue;
            }

            let control_idx = (rng.gen_bool(0.5) as usize) + 1;
            if !placed_wire_in_gate[gate_idx][control_idx] {
                circuit[gate_idx].wires[control_idx] = i as u32;
                placed_wire_in_gate[gate_idx][control_idx] = true;
                break;
            }

            if !placed_wire_in_gate[gate_idx][3 - control_idx] {
                circuit[gate_idx].wires[3 - control_idx] = i as u32;
                placed_wire_in_gate[gate_idx][3 - control_idx] = true;
                break;
            }
        }
    }

    for gate_idx in 0..N_IN {
        loop {
            let t;
            let c1;
            let c2;
            if placed_wire_in_gate[gate_idx][0] {
                t = circuit[gate_idx].wires[0];
            } else {
                t = rng.gen_range(0..N_PROJ_WIRES) as u32;
            }
            if placed_wire_in_gate[gate_idx][1] {
                c1 = circuit[gate_idx].wires[1];
            } else {
                c1 = rng.gen_range(0..N_PROJ_WIRES) as u32;
            }
            if placed_wire_in_gate[gate_idx][2] {
                c2 = circuit[gate_idx].wires[2];
            } else {
                c2 = rng.gen_range(0..N_PROJ_WIRES) as u32;
            }
            if t != c1 && t != c2 && c1 != c2 {
                circuit[gate_idx].wires = [t, c1, c2];
                circuit[gate_idx].control_func = rng.gen_range(0..Base2GateControlFunc::COUNT);
                break;
            }
        }
    }
}

pub fn sample_random_circuit_unguided<R: Rng, const N_IN: usize, const N_PROJ_WIRES: usize>(
    circuit: &mut [Gate; N_IN],
    rng: &mut R,
) {
    let mut rng0 = repeat_with(|| rng.gen_range(0..N_PROJ_WIRES));

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
        gate.control_func = rng.gen_range(0..Base2GateControlFunc::COUNT);
    });
}
#[cfg(test)]
mod tests {
    use rand::RngCore;
    use rand_chacha::ChaCha8Rng;

    use super::*;

    #[test]
    fn test_replacement() {
        const WIRES: usize = 20;
        let mut rng = ChaCha8Rng::from_entropy();

        let mut ckt = [Gate::default(); 2];

        sample_random_circuit_unguided::<_, 2, 5>(&mut ckt, &mut rng);
        // loop {
        //     sample_random_circuit(&mut ckt, &[[false; 20]; 2], &mut rng);
        //     if (ckt[0].wires[0] == ckt[1].wires[1]
        //         || ckt[0].wires[0] == ckt[1].wires[2]
        //         || ckt[1].wires[0] == ckt[0].wires[1]
        //         || ckt[1].wires[0] == ckt[0].wires[2])
        //         && ckt[0].control_func != 0
        //         && ckt[1].control_func != 0
        //     {
        //         break;
        //     }
        // }
        dbg!(&ckt);

        let res =
            find_replacement_circuit::<_, 2, 5, 11, { 1 << 11 }>(&ckt, WIRES, 1000000000, &mut rng);
        dbg!(res);
    }
}
