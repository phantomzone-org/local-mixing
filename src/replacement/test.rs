use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

use crate::{circuit::Gate, local_mixing::consts::N_OUT_INF};

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
    let mut rng = ChaCha8Rng::from_os_rng();
    let mut avg = 0;
    for _ in 0..n_iter {
        let res = find_replacement_circuit::<_, N_OUT_INF>(
            &circuit,
            num_wires,
            1_000_000_000,
            strategy,
            &mut rng,
        );
        match res {
            None => log::error!("replacement failed"),
            Some((replacement, n_sampled)) => {
                avg += n_sampled;
                log::info!("n_sampled = {}, replacement = {:?}", n_sampled, replacement);
            }
        }
    }
    avg /= n_iter;
    println!(
        "Average number of samples: {:?}, over {} iterations",
        avg, n_iter
    );
}
