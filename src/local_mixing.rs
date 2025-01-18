use hashbrown::HashSet;
use rand::{Rng, RngCore};

use crate::circuit::Circuit;

#[derive(Clone)]
pub struct LocalMixingConfig {
    pub original_circuit: Circuit,
    pub num_wires: u32,
    pub inflationary_steps: usize,
    pub kneading_steps: usize,
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

            let mut path_connected_target_wires = vec![false; num_wires];
            let mut path_connected_control_wires = vec![false; num_wires];
            let mut target_count = 0;
            let mut control_count = 0;

            for i in gate_one_idx + 1..num_gates {
                let curr_gate = &self.current_circuit.gates[i];
                let curr_target = curr_gate.target as usize;
                let curr_control0 = curr_gate.control[0] as usize;
                let curr_control1 = curr_gate.control[1] as usize;

                if path_connected_control_wires[curr_target]
                    || path_connected_target_wires[curr_control0]
                    || path_connected_target_wires[curr_control1]
                {
                    if !path_connected_target_wires[curr_target] {
                        path_connected_target_wires[curr_target] = true;
                        target_count += 1;
                    }

                    if !path_connected_control_wires[curr_control0] {
                        path_connected_control_wires[curr_control0] = true;
                        control_count += 1;
                    }

                    if !path_connected_control_wires[curr_control1] {
                        path_connected_control_wires[curr_control1] = true;
                        control_count += 1;
                    }

                    gate_one_successors.insert(i);
                } else if gate_one.collides_with(curr_gate) {
                    candidate_second_idxs.push(i);
                    gate_one_successors.insert(i);

                    if !path_connected_target_wires[curr_target] {
                        path_connected_target_wires[curr_target] = true;
                        target_count += 1;
                    }

                    if !path_connected_control_wires[curr_control0] {
                        path_connected_control_wires[curr_control0] = true;
                        control_count += 1;
                    }

                    if !path_connected_control_wires[curr_control1] {
                        path_connected_control_wires[curr_control1] = true;
                        control_count += 1;
                    }
                }

                if target_count == num_wires || control_count == num_wires {
                    stopping_distance = Some(i - gate_one_idx);
                    break;
                }
            }

            if !candidate_second_idxs.is_empty() {
                break;
            } else {
                gate_one_successors.clear();
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

        // TODO: replacement

        (
            stopping_distance,
            min_candidate_distance,
            max_candidate_distance,
            candidate_second_idxs.len(),
        )
    }

    pub fn run_kneading_step<R: RngCore>(&mut self, rng: &mut R) {
        let num_gates = self.current_circuit.gates.len();
        let num_wires = self.config.num_wires as usize;

        let mut selected_gates: Vec<usize> = vec![0; 5];
        let mut selected_gate_ctr = 0;
        let mut candidate_next_gates: Vec<Vec<usize>> = vec![vec![]; 5];
        let mut candidates_computed = [false; 5];

        while selected_gate_ctr < 5 {
            if selected_gate_ctr != 0 && !candidates_computed[selected_gate_ctr] {
                // compute candidates
                let latest_selected_idx = selected_gates[selected_gate_ctr - 1];
                let latest_selected_gate = &self.current_circuit.gates[latest_selected_idx];

                let mut path_connected_target_wires = vec![false; num_wires];
                let mut path_connected_control_wires = vec![false; num_wires];

                // invariant: |selected_gates| >= 1, and there may be gates before the last inserted gate
                let mut num_selected_gates_seen = 1;
                for i in selected_gates[0] + 1..num_gates {
                    if num_selected_gates_seen < selected_gates.len()
                        && i == selected_gates[num_selected_gates_seen]
                    {
                        num_selected_gates_seen += 1;
                    } else {
                        let curr_gate = &self.current_circuit.gates[i];
                        let curr_target = curr_gate.target as usize;
                        let curr_control0 = curr_gate.control[0] as usize;
                        let curr_control1 = curr_gate.control[1] as usize;

                        let mut collides_with_prev_selected = false;
                        for j in 0..selected_gate_ctr {
                            // iterate over previously selected gates (not latest)
                            // if j < i and they collide
                            let selected_gate = &self.current_circuit.gates[selected_gates[j]];
                            collides_with_prev_selected = collides_with_prev_selected
                                || (j < i && selected_gate.collides_with(curr_gate));
                        }

                        // check collision with path-connected gates
                        if path_connected_control_wires[curr_target]
                            || path_connected_target_wires[curr_control0]
                            || path_connected_target_wires[curr_control1]
                        {
                            // not a candidate, but path-connected
                            path_connected_target_wires[curr_target] = true;
                            path_connected_control_wires[curr_control0] = true;
                            path_connected_control_wires[curr_control1] = true;
                        } else {
                            if latest_selected_gate.collides_with(curr_gate) && latest_selected_idx < i {
                                // candidate
                                candidate_next_gates[selected_gate_ctr].push(i);

                                path_connected_target_wires[curr_target] = true;
                                path_connected_control_wires[curr_control0] = true;
                                path_connected_control_wires[curr_control1] = true;
                            } else if collides_with_prev_selected {
                                path_connected_target_wires[curr_target] = true;
                                path_connected_control_wires[curr_control0] = true;
                                path_connected_control_wires[curr_control1] = true;
                            }
                        }
                    }
                }

                candidates_computed[selected_gate_ctr] = true;
            }

            if selected_gate_ctr == 0 {
                // pick gate 1 at random
                selected_gates[0] = rng.gen_range(0..num_gates - 4);
                selected_gate_ctr += 1;
            } else if candidate_next_gates[selected_gate_ctr].is_empty() {
                // reset candidates for this gate, dec ctr and pick again for prev gate
                candidates_computed[selected_gate_ctr] = false;
                selected_gate_ctr -= 1;
            } else {
                // pick gate from candidates, inc ctr
                let num_candidates = candidate_next_gates[selected_gate_ctr].len();
                selected_gates[selected_gate_ctr] = candidate_next_gates[selected_gate_ctr]
                    .remove(rng.gen_range(0..num_candidates));
                selected_gate_ctr += 1;
            }
        }

        dbg!(&selected_gates);
    }
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

        for _ in 0..iterations {
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
            benchmark_inflationary_step(num_wires, num_gates, 10000);
        }
    }

    #[test]
    fn test_kneading_step() {
        let mut rng = thread_rng();
        let num_wires = 5;
        let num_gates = 20;
        let circuit = Circuit::random(num_wires, num_gates, &mut rng);
        dbg!(circuit.clone());
        let config = LocalMixingConfig {
            original_circuit: circuit.clone(),
            num_wires,
            inflationary_steps: 1,
            kneading_steps: 1,
        };
        let mut job = LocalMixingJob::new(config);
        job.run_kneading_step(&mut rng);
    }
}
