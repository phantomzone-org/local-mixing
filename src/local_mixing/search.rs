use super::LocalMixingJob;
use crate::replacement::find_replacement_circuit;
use rand::{Rng, RngCore, SeedableRng};

impl LocalMixingJob {
    pub fn execute_step<
        R: Send + Sync + RngCore + SeedableRng,
        const N_OUT: usize,
        const N_IN: usize,
        const N_PROJ_WIRES: usize,
        const N_PROJ_INPUTS: usize,
    >(
        &mut self,
        rng: &mut R,
    ) -> bool {
        let num_gates = self.circuit.gates.len();
        let num_wires = self.wires as usize;

        let mut max_candidate_dist = 0;

        let mut selected_gate_idx = [0; N_OUT];
        let mut selected_gate_ctr = 0;
        let mut candidate_next_gates = vec![vec![]; N_OUT];
        let mut candidates_computed = [false; N_OUT];

        let mut search_restart_ctr = 0;

        while selected_gate_ctr < N_OUT {
            if selected_gate_ctr != 0 && !candidates_computed[selected_gate_ctr] {
                // compute candidates
                let latest_selected_idx = selected_gate_idx[selected_gate_ctr - 1];
                let latest_selected_gate = &self.circuit.gates[latest_selected_idx];

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
                        let curr_gate = &self.circuit.gates[i];
                        let curr_target = curr_gate.wires[0] as usize;
                        let curr_control0 = curr_gate.wires[1] as usize;
                        let curr_control1 = curr_gate.wires[2] as usize;

                        let mut collides_with_prev_selected = false;
                        for j in 0..selected_gate_ctr {
                            // iterate over previously selected gates (not latest)
                            // if j < i and they collide
                            let selected_gate = &self.circuit.gates[selected_gate_idx[j]];
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
                if search_restart_ctr >= 1000 {
                    log::warn!("Search has failed 1000 times in a row");
                    search_restart_ctr = 0;
                } else {
                    search_restart_ctr += 1;
                }

                // pick gate 1 at random
                selected_gate_idx[0] = rng.gen_range(0..num_gates - N_OUT + 1);
                selected_gate_ctr += 1;
            } else if candidate_next_gates[selected_gate_ctr].is_empty() {
                // reset candidates for this gate, dec ctr and pick again for prev gate
                candidates_computed[selected_gate_ctr] = false;
                selected_gate_ctr -= 1;
            } else {
                // Save max_candidate_dist
                if selected_gate_ctr == N_OUT - 1 {
                    max_candidate_dist = candidate_next_gates[selected_gate_ctr].last().unwrap()
                        - selected_gate_idx[0];
                }

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
        let selected_gates = std::array::from_fn(|i| self.circuit.gates[selected_gate_idx[i]]);

        let mut path_connected_target_wires = vec![false; num_wires];
        let mut path_connected_control_wires = vec![false; num_wires];

        for j in 0..selected_gate_idx.len() - 1 {
            for i in selected_gate_idx[j] + 1..selected_gate_idx[j + 1] {
                let curr_gate = &self.circuit.gates[i];
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

        let mut write_idx = selected_gate_idx[0];
        for i in 0..to_before.len() {
            self.circuit.gates[write_idx] = to_before[i];
            write_idx += 1;
        }
        let c_out_start = write_idx;
        for i in 0..N_OUT {
            self.circuit.gates[write_idx] = selected_gates[i];
            write_idx += 1;
        }
        let c_out_end = write_idx;
        for i in 0..to_after.len() {
            self.circuit.gates[write_idx] = to_after[i];
            write_idx += 1;
        }

        // replacement
        let c_out = selected_gates;

        let replacement_res = find_replacement_circuit::<_, N_OUT, N_IN, N_PROJ_WIRES, N_PROJ_INPUTS>(
            &c_out,
            num_wires,
            self.max_replacement_samples,
            self.replacement_strategy,
            rng,
        );
        if let Some((c_in, num_sampled)) = replacement_res {
            self.circuit.gates.splice(c_out_start..c_out_end, c_in);

            log::info!(
                "SUCCESS, \
                 n_gates = {:?}, \
                 n_circuits_sampled = {:?}, \
                 max_candidate_dist = {:?}",
                self.circuit.gates.len(),
                num_sampled,
                max_candidate_dist
            );

            return true;
        } else {
            // log::error!("replacement failed, stage = kneading, C_OUT = {:?}", c_out);
            // self.curr_kneading_fail += 1;
        }

        false
    }
}
