use crate::circuit::analysis::projection_circuit;
use crate::circuit::Gate;
use crate::compression::ct::CompressionTable;
use crate::local_mixing::tracer::ReplacementTraceFields;
use rand::seq::{IndexedRandom, SliceRandom};
use rand::Rng;

pub fn find_replacement<R: Rng>(
    circuit: &Vec<Gate>,
    num_wires: usize,
    replacement_size: usize,
    cf_choice: &Vec<u8>,
    ct: &mut CompressionTable,
    rng: &mut R,
) -> Option<(Vec<Gate>, ReplacementTraceFields)> {
    let (proj_circuit, proj_map) = projection_circuit(circuit);
    let circuit_num_wires = proj_map.len();
    if circuit_num_wires > 9 {
        dbg!("circuit_num_wires > 9");
        return None;
    }

    let mut lhs_circuit = proj_circuit.clone();
    let mut replacement_circuit = vec![Gate::default(); replacement_size];

    let mut replacement_idx = 0;
    if replacement_size > 4 {
        // TODO: initial sample to get to regular samples
        dbg!("replacement_size > 4");
        return None;
    }

    let mut num_samples = vec![];

    while replacement_idx < replacement_size {
        num_samples.push(0);
        loop {
            if num_samples[replacement_idx] >= 100000 {
                println!(
                    "exited early, proj_circuit = {:?}, replacement_circuit = {:?}",
                    proj_circuit, replacement_circuit
                );
                return None;
            }
            let g = sample_gate(9, cf_choice, rng);
            num_samples[replacement_idx] += 1;
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
            } else if let Some((_, orig_w)) = proj_map_new_wires.iter().find(|(ww, _)| w == ww) {
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

    Some((
        output_circuit.clone(),
        ReplacementTraceFields {
            input_circuit: circuit.clone(),
            output_circuit: output_circuit,
            num_input_wires: 0,
            num_output_wires: 0,
            num_active_wires: 0,
            min_generation: 0,
            num_circuits_sampled: 0,
        },
    ))
}

fn sample_gate<R: Rng>(num_wires: usize, cf_choice: &Vec<u8>, rng: &mut R) -> Gate {
    let mut wires: Vec<usize> = (0..num_wires).collect();
    wires.shuffle(rng);

    Gate {
        wires: [wires[0], wires[1], wires[2]],
        control_func: cf_choice.choose(rng).copied().unwrap(),
        generation: 0,
    }
}

#[cfg(test)]
mod test {
    use std::time::Instant;

    use rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;

    use crate::{circuit::Gate, compression::ct::CompressionTable};

    use super::find_replacement;

    #[test]
    fn test_replacement_with_ct() {
        println!("loading ct");
        let mut ct = CompressionTable::from_file("bin/table.db");
        println!("done loading ct");
        let mut rng = ChaCha8Rng::from_os_rng();
        let circuit = vec![
            Gate {
                wires: [10, 1, 2],
                control_func: 2,
                generation: 0,
            },
            Gate {
                wires: [1, 3, 4],
                control_func: 9,
                generation: 0,
            },
            Gate {
                wires: [4, 5, 6],
                control_func: 6,
                generation: 0,
            },
            Gate {
                wires: [5, 8, 7],
                control_func: 11,
                generation: 0,
            },
        ];
        let cf_choice = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];
        let replacement_size = 4;

        let s = Instant::now();
        let res = find_replacement(&circuit, 9, replacement_size, &cf_choice, &mut ct, &mut rng);
        let d = Instant::now() - s;
        dbg!(res, d);
    }
}
