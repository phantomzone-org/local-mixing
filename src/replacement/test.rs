use std::time::Instant;

use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

use crate::{
    circuit::Gate,
    local_mixing::consts::{N_IN, N_OUT_INF, N_PROJ_INPUTS, N_PROJ_WIRES},
};

use super::{
    find_replacement_circuit,
    strategy::{ControlFnChoice, ReplacementStrategy},
};

pub fn test_num_samples(strategy: ReplacementStrategy, cf_choice: ControlFnChoice, n_iter: usize) {
    let num_wires = 11;
    let circuit = [
        Gate {
            wires: [0, 1, 2],
            control_func: 4,
        },
        Gate {
            wires: [3, 0, 4],
            control_func: 9,
        },
    ];
    log::info!("input circuit = {:?}", circuit);
    let mut rng = ChaCha8Rng::from_os_rng();
    let mut avg = 0;
    for _ in 0..n_iter {
        let s = Instant::now();
        let res = find_replacement_circuit::<N_OUT_INF, N_IN, N_PROJ_WIRES, N_PROJ_INPUTS, _>(
            &circuit,
            num_wires,
            1_000_000_000,
            strategy,
            cf_choice,
            &mut rng,
        );
        let d = Instant::now() - s;
        match res {
            None => log::error!("replacement failed, n_sampled = 1000000000, time = {:?}", d),
            Some((replacement, n_sampled)) => {
                avg += n_sampled;
                log::info!(
                    "n_sampled = {}, replacement = {:?}, time = {:?}",
                    n_sampled,
                    replacement,
                    d
                );
            }
        }
    }
    avg /= n_iter;
    println!(
        "Average number of samples: {:?}, over {} iterations",
        avg, n_iter
    );
}
