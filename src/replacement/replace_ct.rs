use crate::circuit::analysis::{projection_circuit, truth_table};
use crate::circuit::circuit::{circuit_min_generation, correct_controls};
use crate::circuit::Gate;
use crate::compression::ct::CompressionTable;
use rand::seq::IndexedRandom;
use rand::Rng;

use super::strategy::ControlFnChoice;

pub fn find_replacement<R: Rng>(
    circuit: &[Gate],
    num_wires: usize,
    replacement_size: usize,
    gate_sample_limit: usize,
    ct: &CompressionTable,
    rng: &mut R,
) -> Option<(Vec<Gate>, usize)> {
    let (proj_circuit, proj_map) = projection_circuit(circuit);
    let proj_tt = truth_table(ct.max_wires_supported, &proj_circuit);
    let mut replacement_circuit = Vec::with_capacity(replacement_size);
    let mut num_samples = 0;

    loop {
        let mut curr_num_wires_used = proj_map.len();
        let mut lhs_tt = proj_tt.clone();
        let mut remaining_gates = replacement_size;
        replacement_circuit.clear();

        while remaining_gates > 0 {
            if num_samples >= gate_sample_limit {
                return None;
            }
            let (g, new_curr_num_wires_used) = sample_next_projection_gate(
                curr_num_wires_used,
                ct.max_wires_supported,
                ct.cf_choice,
                rng,
            );
            num_samples += 1;
            let new_lhs_tt = lhs_tt.iter().map(|&x| g.evaluate_usize(x)).collect();
            if let Some(res) = ct.lookup_truth_table(&new_lhs_tt) {
                if res < remaining_gates {
                    lhs_tt = new_lhs_tt;
                    curr_num_wires_used = new_curr_num_wires_used;
                    replacement_circuit.push(g);
                    remaining_gates -= 1;
                }
            }
        }
        replacement_circuit.reverse();

        // map back to original num_wires
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

        if output_circuit.len() == circuit.len()
            && output_circuit.iter().all(|gate| {
                circuit
                    .iter()
                    .any(|g| g.wires == gate.wires && g.control_func == gate.control_func)
            })
        {
            continue;
        }

        // update gate generation
        let min_generation = circuit_min_generation(circuit);
        let new_generation = min_generation + 1;
        output_circuit
            .iter_mut()
            .for_each(|g| g.generation = new_generation);

        return Some((output_circuit, num_samples));
    }
}

#[inline]
fn sample_next_projection_gate<R: Rng>(
    curr_num_wires_used: usize,
    max_num_wires_supported: usize,
    cf_choice: ControlFnChoice,
    rng: &mut R,
) -> (Gate, usize) {
    loop {
        let mut target = rng.random_range(0..max_num_wires_supported);
        let mut control_one = rng.random_range(0..max_num_wires_supported);
        let mut control_two = rng.random_range(0..max_num_wires_supported);
        let mut new_num_wires_used = curr_num_wires_used;

        if target != control_one && target != control_two && control_one != control_two {
            if target >= curr_num_wires_used {
                target = new_num_wires_used;
                new_num_wires_used += 1;
            }
            if control_one >= curr_num_wires_used {
                control_one = new_num_wires_used;
                new_num_wires_used += 1;
            }
            if control_two >= curr_num_wires_used {
                control_two = new_num_wires_used;
                new_num_wires_used += 1;
            }
            return (
                Gate {
                    wires: [target, control_one, control_two],
                    control_func: cf_choice.cfs().choose(rng).copied().unwrap(),
                    generation: 0,
                },
                new_num_wires_used,
            );
        }
    }
}

#[cfg(test)]
mod test {
    use rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;

    use crate::{
        circuit::{circuit::par_check_equiv_probabilistic, Circuit},
        compression::ct::CompressionTable,
        replacement::strategy::ControlFnChoice,
    };

    use super::{find_replacement, sample_next_projection_gate};

    #[test]
    fn test_replacement_ct() {
        let ct = CompressionTable::new(3, 9, ControlFnChoice::TwoBit);
        let wires = 15;
        let mut rng = ChaCha8Rng::from_os_rng();
        let mut replacement_success_count = 0;
        while replacement_success_count < 10 {
            let ckt_one = Circuit::random_with_cf(wires, 2, ControlFnChoice::All, &mut rng).gates;
            let ckt_two = match find_replacement(&ckt_one, wires, 4, 1000000, &ct, &mut rng) {
                Some((r, _)) => {
                    replacement_success_count += 1;
                    r
                }
                None => continue,
            };
            match par_check_equiv_probabilistic(wires, &ckt_one, &ckt_two, 1000, &mut rng) {
                Ok(()) => continue,
                _ => {
                    dbg!(ckt_one);
                    dbg!(ckt_two);
                    panic!();
                }
            }
        }
    }

    #[test]
    fn test_sample_next_projection_gate() {
        let mut rng = rand::rng();
        let g = sample_next_projection_gate(5, 9, ControlFnChoice::TwoBit, &mut rng);
        dbg!(g);
    }
}
