pub mod subgraph;

use std::time::Instant;

use log4rs::append::console::ConsoleAppender;
use rand::Rng;
use rayon::{
    current_num_threads,
    iter::{IndexedParallelIterator, IntoParallelRefMutIterator, ParallelIterator},
};
use subgraph::find_convex_subsets;

use crate::{
    circuit::{circuit::check_equiv_probabilistic, Circuit, Gate},
    compression::ct::{fetch_or_create_compression_table, CompressionTable},
};

pub fn run_compression_strategy_one(circuit_path: &String) {
    let original = Circuit::load_from_json(circuit_path);

    init_logs();

    log::info!("Number of wires: {}", original.num_wires);
    log::info!("Number of gates: {}", original.gates.len());

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
    let mut chunks = split_array_into_approx_chunks(&original.gates, num_threads, &mut rand::rng());

    let optimized_gates = chunks
        .par_iter_mut()
        .enumerate()
        .map(|(worker_id, chunk)| {
            let res = worker_execute(
                worker_id,
                &Circuit {
                    num_wires: original.num_wires,
                    gates: chunk.to_vec(),
                },
            );
            res.gates
        })
        .flatten()
        .collect();

    log::info!("Checking against original...");

    check_equiv_probabilistic(
        original.num_wires,
        &original.gates,
        &optimized_gates,
        10000,
        &mut rand::rng(),
    )
    .expect("Output circuit is not functionally equivalent to original");

    log::info!("Compression finished");
}

// optimize
fn worker_execute(worker_id: usize, input: &Circuit) -> Circuit {
    log::info!(target: &worker_id.to_string(), "worker running");

    let mut ct: CompressionTable<4, 4, { 1 << 4 }> = fetch_or_create_compression_table();
    let mut rng = rand::rng();

    let mut gates = input.gates.clone();

    let subset_size = 300;
    let max_slice = 10;

    loop {
        shuffle_gates_pairwise(&mut gates, 20, &mut rng);
        let num_inner_chunks = 1.max(5.min(gates.len() / 4000));
        let inner_chunk_size = gates.len() / num_inner_chunks;
        gates = gates
            .chunks(inner_chunk_size)
            .map(|chunk| {
                let res = optimize_chunk(
                    Circuit {
                        num_wires: input.num_wires,
                        gates: chunk.to_vec(),
                    },
                    subset_size,
                    max_slice,
                    &mut ct,
                    &mut rng,
                );
                res.gates
            })
            .flatten()
            .collect();
        break;
    }

    Circuit {
        num_wires: input.num_wires,
        gates,
    }
}

// optimizeSubset
fn optimize_chunk<R: Rng>(
    chunk: Circuit,
    subset_size: usize,
    max_slice: usize,
    ct: &mut CompressionTable<4, 4, { 1 << 4 }>,
    rng: &mut R,
) -> Circuit {
    let mut gates = chunk.gates;
    let mut regenerate_graph = true;
    let mut gates_looped = 0;

    while regenerate_graph {
        let s = Instant::now();
        println!("start find_convex_subsets");
        let subsets = find_convex_subsets(subset_size, &gates, rng);
        println!("end find_convex_subsets");
        let d = Instant::now() - s;
        println!("time to compute subsets: {:?}", d);

        regenerate_graph = false;
        gates_looped = 0;

        for subset in subsets {
            let mut slice_to_use = 1;
            gates_looped += 1;
            loop {
                if subset.len() == 0 {
                    break;
                }
                let subset_gates: Vec<Gate> = subset.iter().map(|&i| gates[i]).collect();
                let mut iter = 0;
                let mut got_match = false;
                while iter < max_slice {
                    slice_to_use += 1;
                    if slice_to_use > max_slice {
                        slice_to_use = 2;
                    }
                    if slice_to_use > subset.len() {
                        iter += 1;
                        continue;
                    }
                    // todo: timeToEndWorker()
                    let slice = if slice_to_use <= 6 {
                        slice_to_use
                    } else {
                        // todo: =subset_size?
                        rng.random_range(slice_to_use..subset_size)
                    };
                    // todo: massOptimizeStep
                    mass_optimize_step(&subset_gates, slice, ct);
                }
            }
        }
    }

    Circuit {
        num_wires: chunk.num_wires,
        gates,
    }
}

fn mass_optimize_step(
    selected_gates: &Vec<Gate>,
    slice_size: usize,
    ct: &mut CompressionTable<4, 4, { 1 << 4 }>,
) {
    for i in 0..selected_gates.len() - slice_size {
        let start = i;
        let end = i + slice_size;
        let selected_slice = selected_gates[start..end].to_vec();
        let res = ct.compress_circuit(&selected_slice);
        println!("{:?}", res);
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
