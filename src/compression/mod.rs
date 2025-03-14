pub mod ct;
pub mod subgraph;

use ct::{fetch_or_create_compression_table, CompressionTable};
use rand::Rng;
use rayon::{
    current_num_threads,
    iter::{IntoParallelRefMutIterator, ParallelIterator},
};
use subgraph::{enumerate_subgraphs, successors_predecessors};

use crate::circuit::{circuit::check_equiv_probabilistic, Circuit, Gate};

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

    chunks.par_iter_mut().for_each(|chunk| {
        let old_size = chunk.len();
        StrategyOneRunner::init(chunk.to_vec()).run();
        let new_size = chunk.len();

        dbg!(old_size, new_size);
    });

    let output = Circuit {
        num_wires: input.num_wires,
        gates: chunks.iter().flatten().map(|&g| g).collect(),
    };

    let equiv_check = check_equiv_probabilistic(&input, &output, 10000, &mut rand::rng()).unwrap();
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
            let chunk_size = 100;

            optimized = optimized
                .chunks(chunk_size)
                .map(|chunk| self.optimize_subset(10, 0, chunk))
                .flatten()
                .collect();

            self.gates = optimized;

            break;
        }
    }

    fn optimize_subset(
        &mut self,
        subset_size: usize,
        _max_slice: usize,
        gates: &[Gate],
    ) -> Vec<Gate> {
        println!("optimize_subset()");
        let mut optimized = gates.to_vec();
        loop {
            let (succ, pred) = successors_predecessors(&optimized);
            let res = enumerate_subgraphs(
                &optimized,
                &succ,
                &pred,
                subset_size,
                subset_size,
                &self.ct,
            );

            println!("enumerate_subgraphs res: {:?}", res);

            match res {
                None => {
                    // No more changes are possible, return
                    return optimized.to_vec();
                }
                Some((selected_idx, replacement)) => {
                    let replacement_len = replacement.len();
                    let (mut modified_gates, start_idx) =
                        permute_circuit_with_dependency_data(&optimized, &selected_idx, &succ);
                    modified_gates.splice(start_idx..start_idx + selected_idx.len(), replacement);
                    optimized = modified_gates;
                    println!(
                        "optimized {} gates to {} gates",
                        selected_idx.len(),
                        replacement_len
                    );
                }
            }
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

// fn permute_circuit<const N_OUT: usize>(
//     circuit: &mut Circuit,
//     selected_gate_idx: &[usize; N_OUT],
// ) -> usize {
//     // let selected_gates: [Gate; N_OUT] = std::array::from_fn(|i| circuit.gates[selected_gate_idx[i]]);
//     let mut to_before = vec![];
//     let mut to_after = vec![];
//     let mut path_connected_target_wires = vec![false; circuit.num_wires as usize];
//     let mut path_connected_control_wires = vec![false; circuit.num_wires as usize];

//     for j in 0..selected_gate_idx.len() - 1 {
//         for i in selected_gate_idx[j] + 1..selected_gate_idx[j + 1] {
//             let curr_gate = &circuit.gates[i];
//             let curr_target = curr_gate.wires[0] as usize;
//             let curr_control0 = curr_gate.wires[1] as usize;
//             let curr_control1 = curr_gate.wires[2] as usize;

//             let mut collides_with_prev_selected = false;
//             for k in 0..=j {
//                 collides_with_prev_selected = collides_with_prev_selected
//                     || circuit.gates[selected_gate_idx[k]].collides_with(curr_gate);
//             }

//             if collides_with_prev_selected
//                 || path_connected_control_wires[curr_target]
//                 || path_connected_target_wires[curr_control0]
//                 || path_connected_target_wires[curr_control1]
//             {
//                 to_after.push(*curr_gate);

//                 path_connected_target_wires[curr_target] = true;
//                 path_connected_control_wires[curr_control0] = true;
//                 path_connected_control_wires[curr_control1] = true;
//             } else {
//                 to_before.push(*curr_gate);
//             }
//         }
//     }

//     let mut write_idx = selected_gate_idx[0];
//     for i in 0..to_before.len() {
//         circuit.gates[write_idx] = to_before[i];
//         write_idx += 1;
//     }
//     let c_out_start = write_idx;
//     for i in 0..N_OUT {
//         circuit.gates[write_idx] = circuit.gates[selected_gate_idx[i]];
//         write_idx += 1;
//     }
//     for i in 0..to_after.len() {
//         circuit.gates[write_idx] = to_after[i];
//         write_idx += 1;
//     }

//     c_out_start
// }
