use std::{fs::File, io::BufReader};

use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};

use crate::{circuit::Circuit, replacement::strategy::ControlFnChoice};

use super::search::{find_convex_gate_ids2, permute_circuit};

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
    let mut circuit = Circuit::random_with_cf(num_wires, num_gates, &cf, &mut rng);

    if run_parallel {
        test_parallel();
    } else {
        test_sequential(&mut circuit, permute, iterations);
    }

    circuit.save_as_json(format!("{}/circuit.json", test_dir));
    circuit.save_generation_data(format!("{}/generation.json", test_dir));
}

fn test_parallel() {}

fn test_sequential(circuit: &mut Circuit, permute: bool, iterations: usize) {
    let mut rng = ChaCha8Rng::from_os_rng();

    for _ in 1..=iterations {
        let (selected_gate_idx, _) = find_convex_gate_ids2::<4, _>(&circuit, &mut rng);
        selected_gate_idx
            .iter()
            .for_each(|&id| circuit.gates[id].generation += 1);

        if permute {
            permute_circuit(circuit, &selected_gate_idx);
        }
    }
}
