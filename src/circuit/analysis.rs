use super::Gate;

pub fn projection_circuit<const SIZE: usize, const PROJ_WIRES: usize>(
    circuit: &[Gate; SIZE],
) -> ([Gate; SIZE], [Option<u32>; PROJ_WIRES]) {
    let mut proj_circuit = [Gate::default(); SIZE];
    let mut proj_map = [None; PROJ_WIRES];
    let mut proj_ctr = 0;

    for i in 0..SIZE {
        for w in 0..3 {
            let wire = circuit[i].wires[w];
            if let Some(pos) = proj_map.iter().position(|&x| x == Some(wire)) {
                proj_circuit[i].wires[w] = pos as u32;
            } else {
                proj_map[proj_ctr] = Some(wire);
                proj_circuit[i].wires[w] = proj_ctr as u32;
                proj_ctr += 1;
            }
        }
        proj_circuit[i].control_func = circuit[i].control_func;
    }

    (proj_circuit, proj_map)
}

/*
 * Assumes that max # wiresw is 32,
 * TT_SIZE == 1 << PROJ_WIRES,
 * Otherwise correctness is not guaranteed.
 */
pub fn truth_table<const SIZE: usize, const TT_SIZE: usize>(
    proj_circuit: &[Gate; SIZE],
) -> [u32; TT_SIZE] {
    std::array::from_fn(|i| {
        let mut input = i as u32;
        proj_circuit.iter().for_each(|g| {
            let a = (input & (1 << g.wires[1])) != 0;
            let b = (input & (1 << g.wires[2])) != 0;
            let x = g.evaluate_cf(a, b);
            input ^= (x as u32) << g.wires[0];
        });
        input
    })
}

pub fn active_wires<const PROJ_WIRES: usize, const TT_SIZE: usize>(
    tt: &[u32; TT_SIZE],
) -> [[bool; PROJ_WIRES]; 2] {
    let mut active_wires = [[false; PROJ_WIRES]; 2];

    for i in 0..TT_SIZE {
        let eval_i = tt[i];
        for w in 0..PROJ_WIRES {
            // active target wires
            if (eval_i & (1 << w)) != (i & (1 << w)) as u32 {
                active_wires[0][w] = true;
            }

            // active control wires
            let bit_mask = 1 << w;
            let i_flipped = i ^ bit_mask;
            let eval_i_flipped = tt[i_flipped];
            let disagreement_bits = (eval_i ^ eval_i_flipped) & !(bit_mask as u32);
            if disagreement_bits != 0 {
                active_wires[1][w] = true;
            }
        }
    }

    active_wires
}

#[cfg(test)]
mod tests {
    use crate::circuit::Gate;

    use super::{active_wires, projection_circuit, truth_table};

    #[test]
    fn test_projection_circuit() {
        let non_proj_circuit = [
            Gate {
                wires: [41, 42, 43],
                control_func: 3,
            },
            Gate {
                wires: [43, 40, 41],
                control_func: 9,
            },
        ];
        let expected_proj_circuit = [
            Gate {
                wires: [0, 1, 2],
                control_func: 3,
            },
            Gate {
                wires: [2, 3, 0],
                control_func: 9,
            },
        ];
        let expected_proj_map = [Some(41), Some(42), Some(43), Some(40), None, None];
        let res = projection_circuit(&non_proj_circuit);
        assert_eq!(res.0, expected_proj_circuit);
        assert_eq!(res.1, expected_proj_map);
    }

    #[test]
    fn test_truth_table() {
        let proj_circuit = [
            Gate {
                wires: [0, 1, 2],
                control_func: 3,
            },
            Gate {
                wires: [1, 3, 4],
                control_func: 9,
            },
        ];
        let expected_tt = [
            2, 3, 1, 0, 6, 7, 5, 4, 8, 9, 11, 10, 12, 13, 15, 14, 16, 17, 19, 18, 20, 21, 23, 22,
            26, 27, 25, 24, 30, 31, 29, 28, 34, 35, 33, 32, 38, 39, 37, 36, 40, 41, 43, 42, 44, 45,
            47, 46, 48, 49, 51, 50, 52, 53, 55, 54, 58, 59, 57, 56, 62, 63, 61, 60,
        ];
        let tt = truth_table::<2, 64>(&proj_circuit);
        assert_eq!(tt, expected_tt);
    }

    #[test]
    fn test_active_wires() {
        let proj_circuit = [
            Gate {
                wires: [0, 1, 2],
                control_func: 3,
            },
            Gate {
                wires: [1, 3, 4],
                control_func: 9,
            },
        ];
        let expected_active_wires = [
            [true, true, false, false, false, false],
            [false, true, false, true, true, false],
        ];
        let tt = truth_table::<2, 64>(&proj_circuit);
        let active_wires = active_wires::<6, 64>(&tt);
        assert_eq!(active_wires, expected_active_wires);
    }
}
