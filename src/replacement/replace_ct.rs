use crate::circuit::analysis::{
    compute_active_wires, num_active_wires, projection_circuit, truth_table,
};
use crate::circuit::circuit::correct_controls;
use crate::circuit::Gate;
use crate::compression::ct::CompressionTable;
use crate::local_mixing::consts::ALL_BITLINES;
use crate::local_mixing::tracer::ReplacementTraceFields;
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
) -> Option<(Vec<Gate>, ReplacementTraceFields)> {
    let (proj_circuit, proj_map) = projection_circuit(circuit);
    let tt = truth_table(proj_map.len(), &proj_circuit);
    let active_wires_vecs = compute_active_wires(proj_map.len(), &tt);
    let num_active_wires = num_active_wires(proj_map.len(), active_wires_vecs);

    // Search input CC set will always have <= 9 active wires
    if num_active_wires > 9 {
        dbg!("num_active_wires > 9");
        return None;
    }

    if replacement_size > 4 {
        // TODO: initial sample to get to regular samples
        dbg!("replacement_size > 4");
        return None;
    }

    let mut input_distinct = vec![];
    proj_circuit.iter().for_each(|g| {
        g.wires.iter().for_each(|w| {
            if !input_distinct.contains(w) {
                input_distinct.push(*w);
            }
        })
    });

    let mut lhs_circuit = proj_circuit.clone();
    let mut replacement_circuit = vec![Gate::default(); replacement_size];
    let mut replacement_idx = 0;
    let mut num_samples = 0;
    loop {
        while replacement_idx < replacement_size {
            loop {
                if num_samples >= gate_sample_limit {
                    return None;
                }
                let g = sample_gate(ct.cf_choice, rng);
                num_samples += 1;
                let mut new_lhs = lhs_circuit.clone();
                new_lhs.push(g);
                if let Some(res) = ct.lookup_cxity(&new_lhs) {
                    if res <= replacement_size - replacement_idx - 1 {
                        lhs_circuit = new_lhs;
                        replacement_circuit[replacement_size - replacement_idx - 1] = g;
                        replacement_idx += 1;
                        break;
                    }
                }
            }
        }

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
            replacement_idx = 0;
            lhs_circuit = proj_circuit.clone();
            continue;
        }

        // update gate generation
        let min_generation = circuit.iter().map(|g| g.generation).min().unwrap_or(0);
        let new_generation = min_generation + 1;
        output_circuit
            .iter_mut()
            .for_each(|g| g.generation = new_generation);

        // output distinct wires
        let mut output_distinct = vec![];
        output_circuit.iter().for_each(|g| {
            g.wires.iter().for_each(|w| {
                if !output_distinct.contains(w) {
                    output_distinct.push(*w);
                }
            });
        });

        return Some((
            output_circuit,
            ReplacementTraceFields {
                num_input_wires: input_distinct.len(),
                num_output_wires: output_distinct.len(),
                num_active_wires,
                min_generation,
                num_circuits_sampled: num_samples,
            },
        ));
    }
}

#[inline]
fn sample_gate<R: Rng>(cf_choice: ControlFnChoice, rng: &mut R) -> Gate {
    Gate {
        wires: ALL_BITLINES.choose(rng).copied().unwrap(),
        control_func: cf_choice.cfs().choose(rng).copied().unwrap(),
        generation: 0,
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

    use super::find_replacement;

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
}
