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

        if is_weakly_connected(&candidate_ckt)
            && is_func_equivalent(gates, &candidate_ckt, num_wires as usize)
        {
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
    num_wires: usize,
) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use rand::thread_rng;

    use super::*;
    use crate::circuit::Gate;

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
