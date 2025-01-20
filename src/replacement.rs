use rand::Rng;

use crate::circuit::Gate;

pub fn find_replacement_circuit<R: Rng, const N: usize, const N2: usize>(
    gates: &[Gate; N],
    num_wires: u32,
    num_attempts: usize,
    rng: &mut R,
) -> Option<[Gate; N2]> {
    for _ in 0..num_attempts {
        // gen random circuit
        let candidate_ckt: [Gate; N2] = sample_random_circuit(num_wires, rng);

        if is_weakly_connected(&candidate_ckt) && is_func_equivalent(gates, &candidate_ckt) {
            Some(candidate_ckt);
        }
    }

    None
}

pub fn sample_random_circuit<R: Rng, const N2: usize>(num_wires: u32, rng: &mut R) -> [Gate; N2] {
    let mut gates = [Gate {
        target: 0,
        control: [0, 0],
        control_func: 0,
    }; N2];

    for i in 0..N2 {
        loop {
            let target = rng.gen_range(0..num_wires);
            let control_one = rng.gen_range(0..num_wires);
            let control_two = rng.gen_range(0..num_wires);

            if target != control_one && target != control_two && control_one != control_two {
                gates[i] = Gate {
                    target,
                    control: [control_one, control_two],
                    control_func: rng.gen_range(0..16),
                };
                break;
            }
        }
    }

    gates
}

pub fn is_weakly_connected<const N2: usize>(gates: &[Gate; N2]) -> bool {
    let mut visited = [false; N2];
    let mut stack = Vec::with_capacity(N2);
    stack.push(0);
    visited[0] = true;

    while let Some(current) = stack.pop() {
        for i in 0..5 {
            if !visited[i] && gates[current].collides_with(&gates[i]) {
                visited[i] = true;
                stack.push(i);
            }
        }
    }

    visited.iter().all(|&v| v)
}

pub fn is_func_equivalent<const N: usize, const N2: usize>(
    gates_one: &[Gate; N],
    gates_two: &[Gate; N2],
) -> bool {
    let mut wire_to_idx_map = vec![None; 15];
    let mut n_wires = 0;
    let eval_ckt: Vec<&Gate> = gates_one.iter().chain(gates_two.iter().rev()).collect();
    for g in &eval_ckt {
        for w in [g.target, g.control[0], g.control[1]] {
            let mut wire_already_placed = false;
            for i in 0..n_wires {
                if let Some(wire_entry) = wire_to_idx_map[i] {
                    if w == wire_entry {
                        wire_already_placed = true;
                        break;
                    }
                }
            }
            if !wire_already_placed {
                wire_to_idx_map[n_wires] = Some(w);
                n_wires += 1;
            }
        }
    }

    for i in 0..(1 << n_wires) {
        let mut bit_string = vec![false; n_wires];
        for j in 0..n_wires {
            bit_string[j] = (i & (1 << j)) != 0;
        }
        let mut input = bit_string.clone();
        for g in &eval_ckt {
            // TODO: impl control function behavior
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use rand::thread_rng;

    use super::*;
    use crate::circuit::Gate;

    #[test]
    fn test_random() {
        let n_wires = 5;
        for i in 0..(1 << n_wires) {
            let mut bit_string = vec![false; n_wires];
            for j in 0..n_wires {
                bit_string[j] = (i & (1 << j)) != 0;
            }
            // Here you can evaluate the bit_string with the circuits
            dbg!(&bit_string);
        }
    }

    #[test]
    fn test_random_gates() {
        let mut rng = thread_rng();
        let ckt: [Gate; 5] = sample_random_circuit(10, &mut rng);
        dbg!(ckt);
    }

    #[test]
    fn test_is_weakly_connected() {
        let gates = [
            Gate {
                target: 1,
                control: [2, 3],
                control_func: 0,
            },
            Gate {
                target: 2,
                control: [1, 3],
                control_func: 0,
            },
            Gate {
                target: 3,
                control: [1, 2],
                control_func: 0,
            },
            Gate {
                target: 4,
                control: [1, 2],
                control_func: 0,
            },
            Gate {
                target: 5,
                control: [1, 2],
                control_func: 0,
            },
        ];
        assert!(is_weakly_connected(&gates));

        let gates = [
            Gate {
                target: 1,
                control: [2, 3],
                control_func: 0,
            },
            Gate {
                target: 2,
                control: [1, 3],
                control_func: 0,
            },
            Gate {
                target: 3,
                control: [1, 2],
                control_func: 0,
            },
            Gate {
                target: 4,
                control: [5, 6],
                control_func: 0,
            },
            Gate {
                target: 5,
                control: [4, 6],
                control_func: 0,
            },
        ];
        assert!(!is_weakly_connected(&gates));
    }
}
