pub mod ct;
pub mod subgraph;

use ct::{fetch_or_create_compression_table, CompressionTable};
use log4rs::append::console::ConsoleAppender;
use rand::Rng;
use rayon::{
    current_num_threads,
    iter::{IndexedParallelIterator, IntoParallelRefMutIterator, ParallelIterator},
};
use subgraph::{dependency_data, enumerate_subgraphs, permute_around_subgraph};

use crate::circuit::{circuit::check_equiv_probabilistic, Circuit, Gate};

pub fn run_compression_strategy_one(circuit_path: &String) {
    let input = Circuit::load_from_json(circuit_path);

    init_logs();

    // Split input into chunks per worker thread
    fn split_array_into_approx_chunks<R: Rng>(
        gates: &Vec<Gate>,
        n: usize,
        rng: &mut R,
    ) -> Vec<Vec<Gate>> {
        let avg_size = gates.len() / n;
        let min_size = avg_size * 9 / 10;
        let max_size = avg_size * 11 / 10;

        let mut chunks = vec![];
        let mut index = 0;
        for i in 0..n {
            let remaining_chunks = n - i;
            let remaining_gates = gates.len() - index;
            let remaining_avg = remaining_gates / remaining_chunks;
            let max_possible_size = if max_size < remaining_avg {
                max_size
            } else {
                remaining_avg
            };
            let min_possible_size = if min_size > remaining_avg {
                min_size
            } else {
                remaining_avg
            };
            let chunk_size =
                min_possible_size + rng.random_range(0..max_possible_size - min_possible_size + 1);

            chunks.push(gates[index..index + chunk_size].to_vec());
            index += chunk_size;
        }
        chunks
    }

    let num_threads = current_num_threads();
    let mut chunks = split_array_into_approx_chunks(&input.gates, num_threads, &mut rand::rng());

    let optimized_gates = chunks
        .par_iter_mut()
        .enumerate()
        .map(|(worker_id, chunk)| {
            let mut runner = StrategyOneRunner::init(worker_id, chunk.to_vec());
            runner.run();
            runner.gates
        })
        .flatten()
        .collect();

    let output = Circuit {
        num_wires: input.num_wires,
        gates: optimized_gates,
    };

    let equiv_check = check_equiv_probabilistic(
        input.num_wires as usize,
        &input.gates,
        &output.gates,
        10000,
        &mut rand::rng(),
    )
    .unwrap();
    dbg!(equiv_check);
}

pub struct StrategyOneRunner {
    id: usize,
    gates: Vec<Gate>,
    ct: CompressionTable<4, 4, { 1 << 4 }>,
}

impl StrategyOneRunner {
    fn init(worker_id: usize, gates: Vec<Gate>) -> Self {
        Self {
            id: worker_id,
            gates,
            ct: fetch_or_create_compression_table(),
        }
    }

    fn run(&mut self) {
        let mut rng = rand::rng();

        log::info!(target: &format!("thread {}", self.id).to_string(), "thread running");

        loop {
            shuffle_gates_pairwise(&mut self.gates, 20, &mut rng);
            let mut optimized = self.gates.clone();
            let chunk_size = 300;

            optimized = optimized
                .chunks(chunk_size)
                .map(|chunk| self.optimize_subset(100, 10, chunk))
                .flatten()
                .collect();

            self.gates = optimized;

            break;
        }
    }

    fn optimize_subset(
        &mut self,
        subset_size: usize,
        slice_size: usize,
        gates: &[Gate],
    ) -> Vec<Gate> {
        let mut optimized = gates.to_vec();
        loop {
            let dependency_data = dependency_data(gates);
            let res =
                enumerate_subgraphs(gates, &dependency_data, subset_size, slice_size, &self.ct);

            match res {
                None => {
                    // No more changes are possible, return
                    return optimized.to_vec();
                }
                Some((selected_idx, replacement)) => {
                    let replacement_len = replacement.len();
                    let (mut modified_gates, start_idx) =
                        permute_around_subgraph(&optimized, &selected_idx, &dependency_data);
                    modified_gates.splice(
                        start_idx..start_idx + selected_idx.len(),
                        replacement.clone(),
                    );
                    optimized = modified_gates;
                    println!(
                        "optimized {} gates to {} gates",
                        selected_idx.len(),
                        replacement_len
                    );
                    return optimized.to_vec();
                }
            };
        }
    }
}

fn shuffle_gates_pairwise<R: Rng>(gates: &mut Vec<Gate>, iterations: usize, rng: &mut R) {
    for _ in 0..iterations {
        for i in 0..gates.len() - 1 {
            if rng.random_bool(0.5) && !gates[i].collides_with(&gates[i + 1]) {
                gates.swap(i, i + 1);
            }
        }
    }
}

fn init_logs() {
    let stdout = ConsoleAppender::builder().build();

    let mut config_builder = log4rs::Config::builder();

    config_builder = config_builder
        .appender(log4rs::config::Appender::builder().build("trace", Box::new(stdout)));
    config_builder = config_builder.logger(
        log4rs::config::Logger::builder()
            .appender("trace")
            .additive(false)
            .build("trace", log::LevelFilter::Trace),
    );

    let mut root_builder = log4rs::config::Root::builder();

    root_builder = root_builder.appender("trace");

    match config_builder.build(root_builder.build(log::LevelFilter::Trace)) {
        Ok(config) => {
            if let Err(e) = log4rs::init_config(config) {
                eprintln!("Failed to initialize logging: {}", e);
            }
        }
        Err(e) => {
            eprintln!("Failed to build logging configuration: {}", e);
        }
    }
}
