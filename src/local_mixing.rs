use rand::{Rng, RngCore, SeedableRng};
use serde::{Deserialize, Serialize};

use crate::{
    circuit::{Circuit, Gate},
    replacement::{find_replacement_circuit, ReplacementStrategy},
};

#[derive(Clone, Serialize, Deserialize)]
pub struct LocalMixingConfig {
    pub original_circuit: Circuit,
    pub num_wires: u32,
    pub num_inflationary_steps: usize,
    pub num_kneading_steps: usize,
    pub num_replacement_attempts: usize,
    pub num_inflationary_to_fail: usize,
    pub num_kneading_to_fail: usize,
    pub epoch_size: usize,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct LocalMixingJob {
    pub config: LocalMixingConfig,
    /// Original circuit
    original_circuit: Circuit,
    /// Current obfuscated circuit
    curr_circuit: Circuit,
    pub curr_inflationary_step: usize,
    pub curr_kneading_step: usize,
    pub curr_kneading_fail: usize,
}

impl LocalMixingJob {
    pub fn from_config(config: LocalMixingConfig) -> Self {
        Self {
            config: config.clone(),
            original_circuit: config.original_circuit.clone(),
            curr_circuit: config.original_circuit,
            curr_inflationary_step: 1,
            curr_kneading_step: 1,
            curr_kneading_fail: 0,
        }
    }

    pub fn run_inflationary_step<R: Send + Sync + RngCore + SeedableRng>(&mut self, rng: &mut R) {
        let num_gates = self.curr_circuit.gates.len();
        let num_wires = self.config.num_wires as usize;

        let mut gate_one_idx;
        let mut candidate_second_idxs = vec![];

        let mut num_search_failed = 0;
        let mut num_replacement_failed = 0;

        for iter in 0..self.config.num_inflationary_to_fail {
            gate_one_idx = rng.gen_range(0..num_gates - 1);
            let gate_one = &self.curr_circuit.gates[gate_one_idx];

            let mut path_connected_target_wires = vec![false; num_wires];
            let mut path_connected_control_wires = vec![false; num_wires];
            let mut target_count = 0;
            let mut control_count = 0;

            for i in gate_one_idx + 1..num_gates {
                let curr_gate = &self.curr_circuit.gates[i];
                let curr_target = curr_gate.wires[0] as usize;
                let curr_control0 = curr_gate.wires[1] as usize;
                let curr_control1 = curr_gate.wires[2] as usize;

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
                } else if gate_one.collides_with(curr_gate) {
                    candidate_second_idxs.push(i);

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
                    // log::info!(
                    //     "all wires hit: target_count = {:?}, control_count = {:?}, curr_dist = {:?}",
                    //     target_count,
                    //     control_count,
                    //     i - gate_one_idx,
                    // );
                    break;
                }
            }

            if candidate_second_idxs.is_empty() {
                log::error!(
                    "c_out search failed, gate_one_pos = {:?}, gate_one = {:?}, target_count = {:?}, control_count = {:?}",
                    gate_one_idx,
                    self.curr_circuit.gates[gate_one_idx],
                    target_count,
                    control_count,
                );
                num_search_failed += 1;
                continue;
            }

            let mut gate_two_idx =
                candidate_second_idxs[rng.gen_range(0..candidate_second_idxs.len())];

            // permute step
            let mut to_before = vec![];
            let mut to_after = vec![];
            let gate_one = self.curr_circuit.gates[gate_one_idx];
            let gate_two = self.curr_circuit.gates[gate_two_idx];

            let mut path_connected_target_wires = vec![false; num_wires];
            let mut path_connected_control_wires = vec![false; num_wires];
            for i in gate_one_idx + 1..gate_two_idx {
                let curr_gate = &self.curr_circuit.gates[i];
                let curr_target = curr_gate.wires[0] as usize;
                let curr_control0 = curr_gate.wires[1] as usize;
                let curr_control1 = curr_gate.wires[2] as usize;

                if gate_one.collides_with(&curr_gate)
                    || path_connected_control_wires[curr_target]
                    || path_connected_target_wires[curr_control0]
                    || path_connected_control_wires[curr_control1]
                {
                    to_after.push(*curr_gate);

                    path_connected_target_wires[curr_target] = true;
                    path_connected_control_wires[curr_control0] = true;
                    path_connected_control_wires[curr_control1] = true;
                } else {
                    to_before.push(*curr_gate);
                }
            }
            // copy gate data into circuit s.t. [gate_one, gate_two] are consecutive
            let mut write_idx = gate_one_idx;
            for i in 0..to_before.len() {
                self.curr_circuit.gates[write_idx] = to_before[i];
                write_idx += 1;
            }
            self.curr_circuit.gates[write_idx] = gate_one;
            self.curr_circuit.gates[write_idx + 1] = gate_two;
            let orig_gate_one_idx = gate_one_idx;
            let orig_gate_two_idx = gate_two_idx;
            gate_one_idx = write_idx;
            gate_two_idx = write_idx + 1;
            write_idx += 2;
            for i in 0..to_after.len() {
                self.curr_circuit.gates[write_idx] = to_after[i];
                write_idx += 1;
            }

            // replacement
            let c_out = [gate_one, gate_two];
            let replacement_res = find_replacement_circuit::<_, 2, 5, 11, { 1 << 11 }>(
                &c_out,
                num_wires,
                self.config.num_replacement_attempts,
                ReplacementStrategy::NoID,
                rng,
            );
            if let Some((c_in, replacement_attempts)) = replacement_res {
                log::info!(
                    "stage = inflationary, \
                     step = {:?}, \
                     status = SUCCESS, \
                     n_gates = {:?}, \
                     c_out_ids = {:?}, \
                     candidate_gate_two_set = {:?}, \
                     c_out = {:?}, \
                     max_candidate_two_dist = {:?}, \
                     n_circuits_sampled = {:?}, \
                     n_iter = {:?}, \
                     n_search_failed = {:?}, \
                     n_replacement_failed = {:?}",
                    self.curr_inflationary_step,
                    self.curr_circuit.gates.len(),
                    [orig_gate_one_idx, orig_gate_two_idx],
                    candidate_second_idxs,
                    c_out,
                    candidate_second_idxs[candidate_second_idxs.len() - 1] - orig_gate_one_idx,
                    replacement_attempts,
                    iter,
                    num_search_failed,
                    num_replacement_failed,
                );
                log::info!(
                    "[TEMP]
                        cout = {:?} \
                        cin = {:?}
                    ",
                    c_out,
                    c_in
                );

                self.curr_circuit
                    .gates
                    .splice(gate_one_idx..=gate_two_idx, c_in);

                // done with inflationary step
                self.curr_inflationary_step += 1;
                return;
            } else {
                // log::error!(
                //     "replacement failed, stage = inflationary, C_OUT = {:?}",
                //     c_out
                // );
                num_replacement_failed += 1;
            }
        }

        log::error!(
            "stage = inflationary, step = {:?}, status = FAIL, n_search_failed: {:?}, n_replacement_failed: {:?}",
            self.curr_inflationary_step,
            num_search_failed,
            num_replacement_failed,
        );
    }

    pub fn run_kneading_step<R: Send + Sync + RngCore + SeedableRng>(&mut self, rng: &mut R) {
        let num_gates = self.curr_circuit.gates.len();
        let num_wires = self.config.num_wires as usize;

        let mut selected_gate_idx: Vec<usize> = vec![0; 5];
        let mut selected_gate_ctr = 0;
        let mut candidate_next_gates: Vec<Vec<usize>> = vec![vec![]; 5];
        let mut candidates_computed = [false; 5];

        while selected_gate_ctr < 5 {
            if selected_gate_ctr != 0 && !candidates_computed[selected_gate_ctr] {
                // compute candidates
                let latest_selected_idx = selected_gate_idx[selected_gate_ctr - 1];
                let latest_selected_gate = &self.curr_circuit.gates[latest_selected_idx];

                let mut path_connected_target_wires = vec![false; num_wires];
                let mut path_connected_control_wires = vec![false; num_wires];
                let mut target_count = 0;
                let mut control_count = 0;

                // invariant: |selected_gate_idx| >= 1, and there may be gates before the last inserted gate
                let mut num_selected_gates_seen = 1;
                for i in selected_gate_idx[0] + 1..num_gates {
                    if num_selected_gates_seen < selected_gate_idx.len()
                        && i == selected_gate_idx[num_selected_gates_seen]
                    {
                        num_selected_gates_seen += 1;
                    } else {
                        let curr_gate = &self.curr_circuit.gates[i];
                        let curr_target = curr_gate.wires[0] as usize;
                        let curr_control0 = curr_gate.wires[1] as usize;
                        let curr_control1 = curr_gate.wires[2] as usize;

                        let mut collides_with_prev_selected = false;
                        for j in 0..selected_gate_ctr {
                            // iterate over previously selected gates (not latest)
                            // if j < i and they collide
                            let selected_gate = &self.curr_circuit.gates[selected_gate_idx[j]];
                            collides_with_prev_selected = collides_with_prev_selected
                                || (j < i && selected_gate.collides_with(curr_gate));
                        }

                        // check collision with path-connected gates
                        if path_connected_control_wires[curr_target]
                            || path_connected_target_wires[curr_control0]
                            || path_connected_target_wires[curr_control1]
                        {
                            // not a candidate, but path-connected
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
                        } else {
                            if latest_selected_gate.collides_with(curr_gate)
                                && latest_selected_idx < i
                            {
                                // candidate
                                candidate_next_gates[selected_gate_ctr].push(i);

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
                            } else if collides_with_prev_selected {
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
                        }
                    }

                    if target_count == num_wires || control_count == num_wires {
                        break;
                    }
                }

                candidates_computed[selected_gate_ctr] = true;
            }

            if selected_gate_ctr == 0 {
                // pick gate 1 at random
                selected_gate_idx[0] = rng.gen_range(0..num_gates - 4);
                selected_gate_ctr += 1;
            } else if candidate_next_gates[selected_gate_ctr].is_empty() {
                // reset candidates for this gate, dec ctr and pick again for prev gate
                candidates_computed[selected_gate_ctr] = false;
                selected_gate_ctr -= 1;
            } else {
                // pick gate from candidates, inc ctr
                let num_candidates = candidate_next_gates[selected_gate_ctr].len();
                selected_gate_idx[selected_gate_ctr] = candidate_next_gates[selected_gate_ctr]
                    .remove(rng.gen_range(0..num_candidates));
                selected_gate_ctr += 1;
            }
        }

        // permute step
        let mut to_before = vec![];
        let mut to_after = vec![];
        let selected_gates: Vec<Gate> = selected_gate_idx
            .iter()
            .map(|&idx| self.curr_circuit.gates[idx])
            .collect();

        let mut path_connected_target_wires = vec![false; num_wires];
        let mut path_connected_control_wires = vec![false; num_wires];

        for j in 0..selected_gate_idx.len() - 1 {
            for i in selected_gate_idx[j] + 1..selected_gate_idx[j + 1] {
                let curr_gate = &self.curr_circuit.gates[i];
                let curr_target = curr_gate.wires[0] as usize;
                let curr_control0 = curr_gate.wires[1] as usize;
                let curr_control1 = curr_gate.wires[2] as usize;

                let mut collides_with_prev_selected = false;
                for k in 0..=j {
                    collides_with_prev_selected =
                        collides_with_prev_selected || selected_gates[k].collides_with(curr_gate);
                }

                if collides_with_prev_selected
                    || path_connected_control_wires[curr_target]
                    || path_connected_target_wires[curr_control0]
                    || path_connected_control_wires[curr_control1]
                {
                    to_after.push(*curr_gate);

                    path_connected_target_wires[curr_target] = true;
                    path_connected_control_wires[curr_control0] = true;
                    path_connected_control_wires[curr_control1] = true;
                } else {
                    to_before.push(*curr_gate);
                }
            }
        }

        let c_out_start_idx = selected_gate_idx[0];
        let mut write_idx = c_out_start_idx;
        for i in 0..to_before.len() {
            self.curr_circuit.gates[write_idx] = to_before[i];
            write_idx += 1;
        }
        for i in 0..selected_gates.len() {
            self.curr_circuit.gates[write_idx] = selected_gates[i];
            write_idx += 1;
        }
        for i in 0..to_after.len() {
            self.curr_circuit.gates[write_idx] = to_after[i];
            write_idx += 1;
        }

        // replacement
        let c_out: [Gate; 5] = [
            self.curr_circuit.gates[c_out_start_idx],
            self.curr_circuit.gates[c_out_start_idx + 1],
            self.curr_circuit.gates[c_out_start_idx + 2],
            self.curr_circuit.gates[c_out_start_idx + 3],
            self.curr_circuit.gates[c_out_start_idx + 4],
        ];

        let replacement_res = find_replacement_circuit::<_, 5, 5, 11, { 1 << 11 }>(
            &c_out,
            num_wires,
            self.config.num_replacement_attempts,
            ReplacementStrategy::NoID,
            rng,
        );
        if let Some((c_in, replacement_attempts)) = replacement_res {
            for i in 0..5 {
                self.curr_circuit.gates[c_out_start_idx + i] = c_in[i];
            }

            self.curr_kneading_step += 1;
        } else {
            // log::error!("replacement failed, stage = kneading, C_OUT = {:?}", c_out);
            self.curr_kneading_fail += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use rand_chacha::ChaCha8Rng;

    use super::*;

    fn benchmark_inflationary_step(num_wires: u32, num_gates: usize, iterations: usize) {
        let mut rng = ChaCha8Rng::from_entropy();

        for _ in 0..iterations {
            let circuit = Circuit::random(num_wires, num_gates, &mut rng);
            let config = LocalMixingConfig {
                original_circuit: circuit,
                num_wires,
                num_inflationary_steps: 1,
                num_kneading_steps: 1,
                num_replacement_attempts: 1000000000,
                num_inflationary_to_fail: 10000,
                num_kneading_to_fail: 10000,
                epoch_size: 0,
            };
            let mut local_mixing_job = LocalMixingJob::from_config(config);
            local_mixing_job.run_inflationary_step(&mut rng);
        }
    }

    #[test]
    fn test_inflationary_step() {
        let mut rng = ChaCha8Rng::from_entropy();
        let num_wires = 16;
        let num_gates = 10;
        let circuit = Circuit::random(num_wires, num_gates, &mut rng);
        let config = LocalMixingConfig {
            original_circuit: circuit.clone(),
            num_wires,
            num_inflationary_steps: 1,
            num_kneading_steps: 1,
            num_replacement_attempts: 1000000000,
            num_inflationary_to_fail: 10000,
            num_kneading_to_fail: 10000,
            epoch_size: 0,
        };
        let mut job = LocalMixingJob::from_config(config);
        job.run_inflationary_step(&mut rng);
    }

    #[test]
    fn test_kneading_step() {
        let mut rng = ChaCha8Rng::from_entropy();
        let num_wires = 10;
        let num_gates = 20;
        let circuit = Circuit::random(num_wires, num_gates, &mut rng);
        let config = LocalMixingConfig {
            original_circuit: circuit.clone(),
            num_wires,
            num_inflationary_steps: 1,
            num_kneading_steps: 1,
            num_replacement_attempts: 1000000000,
            num_inflationary_to_fail: 10000,
            num_kneading_to_fail: 10000,
            epoch_size: 0,
        };
        let mut job = LocalMixingJob::from_config(config);
        job.run_kneading_step(&mut rng);
    }

    #[test]
    fn test_benchmark_inflationary_step() {
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
}
