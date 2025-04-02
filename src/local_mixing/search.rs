use super::tracer::ReplacementTraceFields;
use std::error::Error;
use std::time::Instant;

use super::{consts::N_IN, LocalMixingJob};
use crate::{
    circuit::{Circuit, Gate},
    replacement::{replace_ct::find_replacement, strategy::ReplacementStrategy},
};
use rand::{Rng, RngCore, SeedableRng};

fn find_convex_gate_ids<const N_OUT: usize, R: RngCore>(
    circuit: &Circuit,
    rng: &mut R,
) -> ([usize; N_OUT], usize) {
    #[allow(unused_mut)]
    let mut max_candidate_dist = 0;

    let num_gates = circuit.gates.len();
    let num_wires = circuit.num_wires;

    let mut selected_gate_idx = [0; N_OUT];
    let mut selected_gate_ctr = 0;
    let mut candidate_next_gates = vec![vec![]; N_OUT];
    let mut candidates_computed = [false; N_OUT];

    let mut search_restart_ctr = 0;

    while selected_gate_ctr < N_OUT {
        if selected_gate_ctr != 0 && !candidates_computed[selected_gate_ctr] {
            // compute candidates
            let latest_selected_idx = selected_gate_idx[selected_gate_ctr - 1];
            let latest_selected_gate = &circuit.gates[latest_selected_idx];

            let mut path_connected_target_wires = vec![false; num_wires];
            let mut path_connected_control_wires = vec![false; num_wires];
            let mut target_count = 0;
            let mut control_count = 0;

            // invariant: |selected_gate_idx| >= 1, and there may be gates before the last inserted gate
            let mut num_selected_gates_seen = 1;
            for i in selected_gate_idx[0] + 1..num_gates {
                if num_selected_gates_seen < selected_gate_ctr
                    && i == selected_gate_idx[num_selected_gates_seen]
                {
                    num_selected_gates_seen += 1;
                } else {
                    let curr_gate = &circuit.gates[i];
                    let curr_target = curr_gate.wires[0];
                    let curr_control0 = curr_gate.wires[1];
                    let curr_control1 = curr_gate.wires[2];

                    let mut collides_with_prev_selected = false;
                    for j in 0..selected_gate_ctr {
                        // iterate over previously selected gates (not latest)
                        // if j < i and they collide
                        let selected_gate = &circuit.gates[selected_gate_idx[j]];
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
                        if latest_selected_gate.collides_with(curr_gate) && latest_selected_idx < i
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
            if search_restart_ctr >= 100 {
                #[cfg(feature = "trace")]
                log::warn!(target: "trace", "Search has failed 100 times in a row");
                search_restart_ctr = 0;
            } else {
                search_restart_ctr += 1;
            }

            // pick gate 1 at random
            selected_gate_idx[0] = rng.random_range(0..num_gates - N_OUT + 1);
            selected_gate_ctr += 1;
        } else if candidate_next_gates[selected_gate_ctr].is_empty() {
            // reset candidates for this gate, dec ctr and pick again for prev gate
            candidates_computed[selected_gate_ctr] = false;
            selected_gate_ctr -= 1;
        } else {
            #[cfg(feature = "trace")]
            if selected_gate_ctr == N_OUT - 1 {
                max_candidate_dist =
                    candidate_next_gates[selected_gate_ctr].last().unwrap() - selected_gate_idx[0];
            }

            // pick gate from candidates, inc ctr
            let num_candidates = candidate_next_gates[selected_gate_ctr].len();
            selected_gate_idx[selected_gate_ctr] =
                candidate_next_gates[selected_gate_ctr].remove(rng.random_range(0..num_candidates));
            selected_gate_ctr += 1;
        }
    }

    (selected_gate_idx, max_candidate_dist)
}

fn permute_circuit<const N_OUT: usize>(
    circuit: &mut Circuit,
    selected_gate_idx: &[usize; N_OUT],
) -> usize {
    let mut to_before = vec![];
    let mut to_after = vec![];
    let mut path_connected_target_wires = vec![false; circuit.num_wires];
    let mut path_connected_control_wires = vec![false; circuit.num_wires];

    for j in 0..selected_gate_idx.len() - 1 {
        for i in selected_gate_idx[j] + 1..selected_gate_idx[j + 1] {
            let curr_gate = &circuit.gates[i];
            let curr_target = curr_gate.wires[0];
            let curr_control0 = curr_gate.wires[1];
            let curr_control1 = curr_gate.wires[2];

            let mut collides_with_prev_selected = false;
            for k in 0..=j {
                collides_with_prev_selected = collides_with_prev_selected
                    || circuit.gates[selected_gate_idx[k]].collides_with(curr_gate);
            }

            if collides_with_prev_selected
                || path_connected_control_wires[curr_target]
                || path_connected_target_wires[curr_control0]
                || path_connected_target_wires[curr_control1]
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
        circuit.gates[write_idx] = to_before[i];
        write_idx += 1;
    }
    let c_out_start = write_idx;
    for i in 0..N_OUT {
        circuit.gates[write_idx] = circuit.gates[selected_gate_idx[i]];
        write_idx += 1;
    }
    for i in 0..to_after.len() {
        circuit.gates[write_idx] = to_after[i];
        write_idx += 1;
    }

    c_out_start
}

impl LocalMixingJob {
    pub fn execute_step<R: Send + Sync + RngCore + SeedableRng, const N_OUT: usize>(
        &mut self,
        rng: &mut R,
    ) -> Result<(), Box<dyn Error>> {
        #[cfg(feature = "trace")]
        let start_time = Instant::now();

        let (selected_gate_idx, _max_candidate_dist) =
            find_convex_gate_ids::<N_OUT, _>(&self.circuit, rng);

        // replacement step
        let selected_gates: [Gate; N_OUT] =
            std::array::from_fn(|i| self.circuit.gates[selected_gate_idx[i]]);
        let replacement_res = match self.replacement_strategy == ReplacementStrategy::Dummy {
            true => Some((
                vec![Gate::default(); N_IN],
                ReplacementTraceFields::default(),
            )),
            false => {
                #[cfg(feature = "trace")]
                let repl_start = Instant::now();

                // let res = find_replacement_circuit::<N_OUT, N_IN, N_PROJ_WIRES, N_PROJ_INPUTS, _>(
                //     &selected_gates,
                //     self.wires,
                //     self.max_replacement_samples,
                //     self.replacement_strategy,
                //     self.cf_choice,
                //     rng,
                // );
                let res = find_replacement(
                    &selected_gates.to_vec(),
                    self.wires,
                    4,
                    &self.cf_choice.cfs(),
                    &mut self.ct,
                    rng,
                );

                #[cfg(feature = "trace")]
                self.tracer
                    .add_replacement_time(Instant::now() - repl_start);

                res
            }
        };
        if let Some((c_in, replacement_fields)) = replacement_res {
            // permute step
            let c_out_start = permute_circuit(&mut self.circuit, &selected_gate_idx);
            self.circuit
                .gates
                .splice(c_out_start..c_out_start + N_OUT, c_in);

            #[cfg(feature = "trace")]
            self.tracer.add_search_entry(
                self.circuit.gates.len(),
                _max_candidate_dist,
                Instant::now() - start_time,
                replacement_fields,
            );

            return Ok(());
        }

        Err(format!(
            "Failed to find replacement for c_out = {:?}",
            selected_gates
        )
        .into())
    }
}

#[cfg(test)]
mod tests {
    use crate::{circuit::Circuit, local_mixing::consts::N_OUT_KND};

    use super::find_convex_gate_ids;

    fn is_convex(circuit: &Circuit, convex_gate_ids: &[usize]) -> bool {
        let mut is_convex = true;

        let mut colliding_set = vec![];
        let mut path_colliding_targets = vec![false; circuit.num_wires];
        let mut path_colliding_controls = vec![false; circuit.num_wires];
        'outer: for i in convex_gate_ids[0]..*convex_gate_ids.last().unwrap() + 1 {
            if convex_gate_ids.contains(&i) {
                let selected_gate = circuit.gates[i];
                // check no collision with any gate in colliding_set
                for c_gate in colliding_set.iter() {
                    if selected_gate.collides_with(c_gate) {
                        is_convex = false;
                        break 'outer;
                    }
                }

                let [t, c0, c1] = circuit.gates[i].wires;
                path_colliding_targets[t] = true;
                path_colliding_controls[c0] = true;
                path_colliding_controls[c1] = true;
            } else {
                let [t, c0, c1] = circuit.gates[i].wires;
                if path_colliding_targets[c0]
                    || path_colliding_targets[c1]
                    || path_colliding_controls[t]
                {
                    colliding_set.push(circuit.gates[i].clone());
                    path_colliding_targets[t] = true;
                    path_colliding_controls[c0] = true;
                    path_colliding_controls[c1] = true;
                }
            }
        }

        is_convex
    }

    #[test]
    fn test_find_convex() {
        let num_wires = 64;
        let num_gates = 10000;
        let mut rng = rand::rng();
        for i in 0..1000 {
            let circuit = Circuit::random(num_wires, num_gates, &mut rng);
            let (convex_gate_ids, _) = find_convex_gate_ids::<N_OUT_KND, _>(&circuit, &mut rng);
            assert!(
                is_convex(&circuit, &convex_gate_ids),
                "failed at iteration {i}"
            );
        }
    }
}
