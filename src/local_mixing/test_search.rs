use std::{fs::File, io::BufReader};

use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use rayon::{
    current_num_threads,
    iter::{IndexedParallelIterator, IntoParallelRefMutIterator, ParallelIterator},
    slice::ParallelSliceMut,
};
use serde::{Deserialize, Serialize};

use crate::{circuit::Circuit, replacement::strategy::ControlFnChoice};

use super::search::{find_convex_gate_ids3, permute_circuit};

#[derive(Serialize, Deserialize)]
pub struct SearchTestConfig {
    num_wires: usize,
    num_gates: usize,
    cf: ControlFnChoice,
    run_parallel: bool,
    permute: bool,
    iterations: usize,
}

pub fn test_local_mixing_search(test_dir: &str) {
    let config_path = format!("{}/config.json", test_dir);
    let config: SearchTestConfig =
        serde_json::from_reader(BufReader::new(File::open(&config_path).unwrap())).unwrap();

    let SearchTestConfig {
        num_wires,
        num_gates,
        cf,
        run_parallel,
        permute,
        iterations,
    } = config;

    let mut rng = ChaCha8Rng::from_os_rng();
    let mut circuit = Circuit::random_with_cf(num_wires, num_gates, cf, &mut rng);

    if run_parallel {
        test_parallel(&mut circuit, permute, iterations);
    } else {
        test_sequential(&mut circuit, permute, iterations);
    }

    circuit.save_as_json(format!("{}/circuit.json", test_dir));
    circuit.save_generation_data(format!("{}/generation.json", test_dir));
}

fn test_parallel(circuit: &mut Circuit, permute: bool, iterations: usize) {
    let num_search_workers = current_num_threads();
    println!("-- Using {} search workers", num_search_workers);

    let num_gates = circuit.gates.len();
    let chunk_size = num_gates / num_search_workers;
    let mut rngs: Vec<_> = (0..num_search_workers)
        .map(|_| ChaCha8Rng::from_os_rng())
        .collect();

    let mut knd_steps = 0;
    while knd_steps < iterations {
        // Phase 1: even chunks
        circuit
            .gates
            .par_chunks_mut(chunk_size)
            .zip_eq(rngs.par_iter_mut())
            .for_each(|(chunk, rng)| {
                let (selected_gate_idx, _) =
                    find_convex_gate_ids3::<4, _>(circuit.num_wires, &chunk, rng);
                selected_gate_idx
                    .iter()
                    .for_each(|&id| chunk[id].generation += 1);

                if permute {
                    permute_circuit(circuit.num_wires, chunk, &selected_gate_idx);
                }
            });

        // Phase 2: |1st chunk| = chunk_size / 2, |last chunk| = chunk_size * 3 / 2
        let mut chunks = vec![];
        let (first, rest) = circuit.gates.split_at_mut(chunk_size / 2);
        let (middle, last) = rest.split_at_mut(num_gates - chunk_size * 2);
        chunks.push(first);
        middle
            .chunks_mut(chunk_size)
            .for_each(|chunk| chunks.push(chunk));
        chunks.push(last);

        chunks
            .par_iter_mut()
            .zip_eq(rngs.par_iter_mut())
            .for_each(|(chunk, rng)| {
                let (selected_gate_idx, _) =
                    find_convex_gate_ids3::<4, _>(circuit.num_wires, &chunk, rng);
                selected_gate_idx
                    .iter()
                    .for_each(|&id| chunk[id].generation += 1);

                if permute {
                    permute_circuit(circuit.num_wires, chunk, &selected_gate_idx);
                }
            });

        // Phase 3: |1st chunk| = chunk_size * 3 / 2, |last chunk| = chunk_size / 2
        let mut chunks = vec![];
        let (first, rest) = circuit.gates.split_at_mut(chunk_size * 3 / 2);
        let (middle, last) = rest.split_at_mut(num_gates - chunk_size * 2);
        chunks.push(first);
        middle
            .chunks_mut(chunk_size)
            .for_each(|chunk| chunks.push(chunk));
        chunks.push(last);

        chunks
            .par_iter_mut()
            .zip_eq(rngs.par_iter_mut())
            .for_each(|(chunk, rng)| {
                let (selected_gate_idx, _) =
                    find_convex_gate_ids3::<4, _>(circuit.num_wires, &chunk, rng);
                selected_gate_idx
                    .iter()
                    .for_each(|&id| chunk[id].generation += 1);

                if permute {
                    permute_circuit(circuit.num_wires, chunk, &selected_gate_idx);
                }
            });

        knd_steps += 3 * num_search_workers;
    }
}

fn test_sequential(circuit: &mut Circuit, permute: bool, iterations: usize) {
    let mut rng = ChaCha8Rng::from_os_rng();

    for _ in 1..=iterations {
        let (selected_gate_idx, _) =
            find_convex_gate_ids3::<4, _>(circuit.num_wires, &circuit.gates, &mut rng);
        selected_gate_idx
            .iter()
            .for_each(|&id| circuit.gates[id].generation += 1);

        if permute {
            permute_circuit(circuit.num_wires, &mut circuit.gates, &selected_gate_idx);
        }
    }
}
