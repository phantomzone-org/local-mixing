use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

use crate::circuit::Gate;

use super::{find_replacement_circuit, strategy::ReplacementStrategy};

pub fn test_num_samples(strategy: ReplacementStrategy, n_iter: usize) {
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
    let mut rng = ChaCha8Rng::from_entropy();
    let mut avg = 0;
    for i in 0..n_iter {
        let res = find_replacement_circuit::<_, 2, 4, 9, { 1 << 9 }>(
            &circuit,
            num_wires,
            1_000_000_000,
            strategy,
            &mut rng,
        );
        match res {
            None => log::error!("Iteration {}, replacement failed", i),
            Some((replacement, n_sampled)) => {
                avg += n_sampled;
                log::info!(
                    "Iteration {}, n_sampled = {}, replacement = {:?}",
                    i,
                    n_sampled,
                    replacement
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
