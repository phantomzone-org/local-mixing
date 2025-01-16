use hashbrown::HashSet;
use rand::{Rng, RngCore};

use crate::circuit::Circuit;

#[derive(Clone)]
pub struct LocalMixingConfig {
    original_circuit: Circuit,
    num_wires: u32,
    inflationary_steps: usize,
    kneading_steps: usize,
}

pub struct LocalMixingJob {
    config: LocalMixingConfig,
    current_circuit: Circuit,
}

impl LocalMixingJob {
    pub fn new(config: LocalMixingConfig) -> Self {
        Self {
            config: config.clone(),
            current_circuit: config.original_circuit,
        }
    }

    pub fn run_inflationary_step<R: RngCore>(
        &mut self,
        rng: &mut R,
    ) -> (Option<usize>, usize, usize, usize) {
        let num_gates = self.current_circuit.gates.len();
        let num_wires = self.config.num_wires as usize;
        let mut stopping_distance = None;

        let mut gate_one_idx;
        let mut candidate_second_idxs = vec![];
        let mut gate_one_successors = HashSet::new();

        loop {
            gate_one_idx = rng.gen_range(0..num_gates - 1);
            let gate_one = &self.current_circuit.gates[gate_one_idx];

            gate_one_successors.clear();
            candidate_second_idxs.clear();
            let mut eliminated_target_wires = vec![false; num_wires];
            let mut eliminated_control_wires = vec![false; num_wires];
            let mut eliminated_target_count = 0;
            let mut eliminated_control_count = 0;

            for i in gate_one_idx + 1..num_gates {
                let curr_gate = &self.current_circuit.gates[i];

                let target_idx = curr_gate.target as usize;
                let control0_idx = curr_gate.control[0] as usize;
                let control1_idx = curr_gate.control[1] as usize;

                let target_eliminated = eliminated_control_wires[target_idx];
                let control0_eliminated = eliminated_target_wires[control0_idx];
                let control1_eliminated = eliminated_target_wires[control1_idx];

                if target_eliminated || control0_eliminated || control1_eliminated {
                    if !target_eliminated {
                        eliminated_control_wires[target_idx] = true;
                        eliminated_control_count += 1;
                    }
                    if !control0_eliminated {
                        eliminated_target_wires[control0_idx] = true;
                        eliminated_target_count += 1;
                    }
                    if !control1_eliminated {
                        eliminated_target_wires[control1_idx] = true;
                        eliminated_target_count += 1;
                    }
                    gate_one_successors.insert(i);
                } else if gate_one.collides_with(&curr_gate) {
                    candidate_second_idxs.push(i);

                    if !eliminated_target_wires[target_idx] {
                        eliminated_target_wires[target_idx] = true;
                        eliminated_target_count += 1;
                    }
                    if !eliminated_control_wires[control0_idx] {
                        eliminated_control_wires[control0_idx] = true;
                        eliminated_control_count += 1;
                    }
                    if !eliminated_control_wires[control1_idx] {
                        eliminated_control_wires[control1_idx] = true;
                        eliminated_control_count += 1;
                    }
                    gate_one_successors.insert(i);
                }

                if eliminated_target_count == num_wires || eliminated_control_count == num_wires {
                    stopping_distance = Some(i - gate_one_idx);
                    break;
                }
            }

            if !candidate_second_idxs.is_empty() {
                break;
            }
        }

        let min_candidate_distance = candidate_second_idxs[0] - gate_one_idx;
        let max_candidate_distance =
            candidate_second_idxs[candidate_second_idxs.len() - 1] - gate_one_idx;

        let gate_two_idx = candidate_second_idxs[rng.gen_range(0..candidate_second_idxs.len())];

        let mut to_before = vec![];
        let mut to_after = vec![];
        let num_permuted;
        {
            let between = self
                .current_circuit
                .gates
                .drain((gate_one_idx + 1)..gate_two_idx);
            num_permuted = between.len();
            for (i, gate) in between.enumerate() {
                if gate_one_successors.contains(&(gate_one_idx + 1 + i)) {
                    to_after.push(gate);
                } else {
                    to_before.push(gate);
                }
            }
        }

        let new_gate_two_idx = gate_two_idx - num_permuted + 1;
        self.current_circuit
            .gates
            .splice(new_gate_two_idx..new_gate_two_idx, to_after);
        self.current_circuit
            .gates
            .splice(gate_one_idx..gate_one_idx, to_before);

        (
            stopping_distance,
            min_candidate_distance,
            max_candidate_distance,
            candidate_second_idxs.len(),
        )
    }

    // Maintain set of target (control) wires
    // it should store # times we've seen target/control, when we pass backwards to revert
}

#[cfg(test)]
mod tests {
    use rand::thread_rng;

    use super::*;

    fn benchmark_inflationary_step(num_wires: u32, num_gates: usize, iterations: usize) {
        let mut rng = thread_rng();

        let mut avg_min_candidate_dist = 0;
        let mut avg_max_candidate_dist = 0;
        let mut avg_stopping_dist = 0;
        let mut avg_num_gate_two_candidates = 0;

        for i in 0..iterations {
            let circuit = Circuit::random(num_wires, num_gates, &mut rng);
            let config = LocalMixingConfig {
                original_circuit: circuit,
                num_wires,
                inflationary_steps: 1,
                kneading_steps: 1,
            };
            let mut local_mixing_job = LocalMixingJob::new(config);
            let (stopping_distance, min_candidate_distance, max_candidate_distance, num_candidates) =
                local_mixing_job.run_inflationary_step(&mut rng);

            avg_min_candidate_dist += min_candidate_distance;
            avg_max_candidate_dist += max_candidate_distance;
            avg_num_gate_two_candidates += num_candidates;
            if let Some(sd) = stopping_distance {
                avg_stopping_dist += sd;
            }
        }

        avg_min_candidate_dist /= iterations;
        avg_max_candidate_dist /= iterations;
        avg_stopping_dist /= iterations;
        avg_num_gate_two_candidates /= iterations;

        dbg!(
            (num_wires, num_gates, iterations),
            avg_num_gate_two_candidates,
            avg_min_candidate_dist,
            avg_max_candidate_dist,
            avg_stopping_dist
        );
    }

    #[test]
    fn test_inflationary_step() {
        let jobs = [
            (100, 1000),
            (200, 1000),
            (500, 1000),
            (100, 5000),
            (200, 5000),
            (500, 5000),
            (100, 10000),
            (200, 10000),
            (500, 10000),
            (100, 100000),
            (200, 100000),
            (500, 100000),
        ];

        for (num_wires, num_gates) in jobs {
            benchmark_inflationary_step(num_wires, num_gates, 1000);
        }
    }
}
