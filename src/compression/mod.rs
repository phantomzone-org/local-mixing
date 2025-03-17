pub mod ct;
pub mod subgraph;
pub mod subgraph_new;

use ct::{fetch_or_create_compression_table, CompressionTable};
use rand::Rng;
use rayon::{
    current_num_threads,
    iter::{IntoParallelRefMutIterator, ParallelIterator},
};
use subgraph::{enumerate_subgraphs, successors_predecessors};

use crate::circuit::{circuit::check_equiv_probabilistic, Circuit, Gate};
use serde_json::json;
use std::fs::File;
use std::io::Write;

pub fn run_compression_strategy_one(input: &Circuit) {
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

    let optimized_gates: Vec<Gate> = chunks
        .par_iter_mut()
        .map(|chunk| {
            let mut runner = StrategyOneRunner::init(chunk.to_vec());
            runner.run();
            runner.gates
        })
        .flatten()
        .collect();

    let output = Circuit {
        num_wires: input.num_wires,
        gates: optimized_gates,
    };

    dbg!(output.gates.len());

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
    gates: Vec<Gate>,
    ct: CompressionTable<4, 4, { 1 << 4 }>,
}

impl StrategyOneRunner {
    fn init(gates: Vec<Gate>) -> Self {
        println!("StrategyOneRunner init()");
        Self {
            gates,
            ct: fetch_or_create_compression_table(),
        }
    }

    fn run(&mut self) {
        println!("StrategyOneRunner run()");

        let mut rng = rand::rng();

        loop {
            println!("run() loop");
            shuffle_gates_pairwise(&mut self.gates, 20, &mut rng);
            let mut optimized = self.gates.clone();
            let chunk_size = 300;

            optimized = optimized
                .chunks(chunk_size)
                .map(|chunk| self.optimize_subset(10, 10, chunk))
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
            let (succ, pred) = successors_predecessors(&optimized);
            let res =
                enumerate_subgraphs(&optimized, &succ, &pred, subset_size, slice_size, &self.ct);

            match res {
                None => {
                    // No more changes are possible, return
                    return optimized.to_vec();
                }
                Some((selected_idx, replacement)) => {
                    let replacement_len = replacement.len();
                    let (mut modified_gates, start_idx) =
                        permute_circuit_with_dependency_data(&optimized, &selected_idx, &succ);
                    let res = check_equiv_probabilistic(
                        64,
                        &optimized,
                        &modified_gates,
                        1000,
                        &mut rand::rng(),
                    );
                    if res.is_err() {
                        dbg!("END");
                        std::process::exit(1); // Stop all threads
                    }
                    modified_gates.splice(
                        start_idx..start_idx + selected_idx.len(),
                        replacement.clone(),
                    );
                    // ckt check
                    let res = check_equiv_probabilistic(
                        64,
                        &optimized,
                        &modified_gates,
                        1000,
                        &mut rand::rng(),
                    );
                    if res.is_err() {
                        println!("mismatch before optimized = modified_gates");
                        panic!();
                    }
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

fn permute_circuit_with_dependency_data(
    gates: &[Gate],
    selected_idx: &[usize],
    succ: &Vec<Vec<usize>>,
) -> (Vec<Gate>, usize) {
    let selected_gates: Vec<Gate> = selected_idx.iter().map(|&i| gates[i]).collect();
    let mut to_before = vec![];
    let mut to_after = vec![];

    for j in 0..selected_idx.len() - 1 {
        for i in selected_idx[j] + 1..selected_idx[j + 1] {
            if succ[selected_idx[j]].contains(&i) {
                to_after.push(gates[i]);
            } else {
                to_before.push(gates[i]);
            }
        }
    }

    let mut permuted_gates = gates.to_vec();
    let mut write_idx = selected_idx[0];
    for g in to_before {
        permuted_gates[write_idx] = g;
        write_idx += 1;
    }
    let subgraph_start = write_idx;
    for g in selected_gates {
        permuted_gates[write_idx] = g;
        write_idx += 1;
    }
    for g in to_after {
        permuted_gates[write_idx] = g;
        write_idx += 1;
    }

    (permuted_gates, subgraph_start)
}
