use super::Gate;

/*
 * Relabels the wires of circuit to 0..
 */
pub fn projection_circuit(circuit: &[Gate]) -> (Vec<Gate>, Vec<usize>) {
    let mut proj_circuit = vec![Gate::default(); circuit.len()];
    let mut proj_map = vec![];

    for i in 0..circuit.len() {
        for w in 0..3 {
            let wire = circuit[i].wires[w];
            if let Some(pos) = proj_map.iter().position(|&x| x == wire) {
                proj_circuit[i].wires[w] = pos;
            } else {
                proj_circuit[i].wires[w] = proj_map.len();
                proj_map.push(wire);
            }
        }
        proj_circuit[i].control_func = circuit[i].control_func;
    }

    (proj_circuit, proj_map)
}

/*
 * Computes the truth table over {0, 1}^num_wires.
 * Assumes that proj_circuit has wires labeled 0.. and num_wires < 32
 */
pub fn truth_table(num_wires: usize, proj_circuit: &Vec<Gate>) -> Vec<usize> {
    let mut tt = vec![];
    for i in 0..1 << num_wires {
        let mut input = i;
        proj_circuit
            .iter()
            .for_each(|g| input = g.evaluate_usize(input));
        tt.push(input);
    }
    tt
}

/*
 * Computes the 'active' wires: which bitlines are significant in input/output.
 */
pub fn compute_active_wires(num_wires: usize, tt: &Vec<usize>) -> (Vec<usize>, Vec<usize>) {
    let mut active_wires = vec![[false; 2]; num_wires];
    for i in 0..tt.len() {
        let eval_i = tt[i];
        for w in 0..num_wires {
            // active target wires
            if (eval_i & (1 << w)) != (i & (1 << w)) {
                active_wires[w][0] = true;
            }

            // active control wires
            let bit_mask = 1 << w;
            let i_flipped = i ^ bit_mask;
            let eval_i_flipped = tt[i_flipped];
            let disagreement_bits = (eval_i ^ eval_i_flipped) & !bit_mask;
            if disagreement_bits != 0 {
                active_wires[w][1] = true;
            }
        }
    }

    let mut active_target = vec![];
    let mut active_control = vec![];
    for w in 0..num_wires {
        if active_wires[w][0] {
            active_target.push(w);
        }
        if active_wires[w][1] {
            active_control.push(w);
        }
    }

    (active_target, active_control)
}

pub fn num_active_wires(num_wires: usize, active_wires_vec: (Vec<usize>, Vec<usize>)) -> usize {
    let mut active_wires = vec![false; num_wires];
    let mut num_active_wires = 0;
    for w in active_wires_vec.0 {
        if !active_wires[w] {
            active_wires[w] = true;
            num_active_wires += 1;
        }
    }
    for w in active_wires_vec.1 {
        if !active_wires[w] {
            active_wires[w] = true;
            num_active_wires += 1;
        }
    }
    num_active_wires
}

pub fn optimal_projection_circuit(circuit: &[Gate]) -> (Vec<Gate>, Vec<usize>, usize) {
    let (proj_circuit, proj_map) = projection_circuit(&circuit);
    let num_wires = proj_map.len();
    let tt = truth_table(num_wires, &proj_circuit);
    let active_wires = compute_active_wires(num_wires, &tt);

    let mut updated_proj_circuit = vec![Gate::default(); circuit.len()];
    let mut updated_proj_map = vec![];
    let mut num_active_wires = 0;
    let mut proj_ctr = 0;
    let mut non_active_pos = vec![];

    for i in 0..circuit.len() {
        for w in 0..3 {
            let wire = circuit[i].wires[w];
            let proj_wire = proj_circuit[i].wires[w];
            if let Some(pos) = updated_proj_map.iter().position(|&x| x == wire) {
                updated_proj_circuit[i].wires[w] = pos;
            } else if active_wires.0.contains(&(proj_wire)) || active_wires.1.contains(&(proj_wire))
            {
                updated_proj_circuit[i].wires[w] = proj_ctr;
                updated_proj_map.push(wire);
                num_active_wires += 1;
                proj_ctr += 1;
            } else {
                non_active_pos.push((i, w));
            }
        }
        updated_proj_circuit[i].control_func = circuit[i].control_func;
    }

    non_active_pos.iter().for_each(|&(i, w)| {
        let wire = circuit[i].wires[w];
        if let Some(pos) = updated_proj_map.iter().position(|&x| x == wire) {
            updated_proj_circuit[i].wires[w] = pos;
        } else {
            updated_proj_circuit[i].wires[w] = proj_ctr;
            updated_proj_map.push(wire);
            proj_ctr += 1;
        }
    });

    (updated_proj_circuit, updated_proj_map, num_active_wires)
}

pub fn num_distinct_wires(circuit: &[Gate]) -> usize {
    let mut distinct_wires = vec![];
    for gate in circuit {
        for wire in gate.wires {
            if !distinct_wires.contains(&wire) {
                distinct_wires.push(wire);
            }
        }
    }
    distinct_wires.len()
}

#[cfg(test)]
mod tests {
    use crate::circuit::{analysis::optimal_projection_circuit, Gate};

    use super::{compute_active_wires, projection_circuit, truth_table};

    #[test]
    fn test_projection_circuit() {
        let non_proj_circuit = vec![
            Gate {
                wires: [41, 42, 43],
                control_func: 3,
                generation: 0,
            },
            Gate {
                wires: [43, 40, 41],
                control_func: 9,
                generation: 0,
            },
        ];
        let expected_proj_circuit = [
            Gate {
                wires: [0, 1, 2],
                control_func: 3,
                generation: 0,
            },
            Gate {
                wires: [2, 3, 0],
                control_func: 9,
                generation: 0,
            },
        ];
        let expected_proj_map = [41, 42, 43, 40];
        let res = projection_circuit(&non_proj_circuit);
        assert_eq!(res.0, expected_proj_circuit);
        assert_eq!(res.1, expected_proj_map);
    }

    #[test]
    fn test_truth_table() {
        let proj_circuit = vec![
            Gate {
                wires: [0, 1, 2],
                control_func: 3,
                generation: 0,
            },
            Gate {
                wires: [1, 3, 4],
                control_func: 9,
                generation: 0,
            },
        ];
        let expected_tt = [
            2, 3, 1, 0, 6, 7, 5, 4, 8, 9, 11, 10, 12, 13, 15, 14, 16, 17, 19, 18, 20, 21, 23, 22,
            26, 27, 25, 24, 30, 31, 29, 28, 34, 35, 33, 32, 38, 39, 37, 36, 40, 41, 43, 42, 44, 45,
            47, 46, 48, 49, 51, 50, 52, 53, 55, 54, 58, 59, 57, 56, 62, 63, 61, 60,
        ];
        let tt = truth_table(6, &proj_circuit);
        assert_eq!(tt, expected_tt);
    }

    #[test]
    fn test_active_wires() {
        let proj_circuit = vec![
            Gate {
                wires: [0, 1, 2],
                control_func: 3,
                generation: 0,
            },
            Gate {
                wires: [1, 3, 4],
                control_func: 9,
                generation: 0,
            },
        ];
        let expected_active_wires = (vec![0, 1], vec![1, 3, 4]);
        let tt = truth_table(6, &proj_circuit);
        let active_wires = compute_active_wires(6, &tt);
        assert_eq!(active_wires, expected_active_wires);
    }

    #[test]
    fn test_optimal_projection_circuit() {
        let circuit = vec![
            Gate {
                wires: [33, 22, 60],
                control_func: 15,
                generation: 0,
            },
            Gate {
                wires: [33, 2, 22],
                control_func: 3,
                generation: 0,
            },
            Gate {
                wires: [33, 22, 2],
                control_func: 9,
                generation: 0,
            },
            Gate {
                wires: [55, 12, 24],
                control_func: 15,
                generation: 0,
            },
        ];
        dbg!(&circuit);

        let proj = projection_circuit(&circuit);
        dbg!(proj);

        let opt = optimal_projection_circuit(&circuit);
        dbg!(opt);
    }
}
