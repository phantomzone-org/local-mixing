use crate::circuit::{Circuit, Gate};
use rand::{seq::IndexedRandom, Rng, RngCore};

struct PathConnectedWires {
    wires: Vec<bool>,
    count: usize,
}

impl PathConnectedWires {
    fn new(num_wires: usize) -> Self {
        Self {
            wires: vec![false; num_wires],
            count: 0,
        }
    }

    fn all_wires_hit(&self) -> bool {
        self.count == self.wires.len()
    }

    fn wire_hit(&self, wire: usize) -> bool {
        self.wires[wire]
    }

    fn add_wire(&mut self, wire: usize) {
        if !self.wires[wire] {
            self.count += 1;
        }
        self.wires[wire] = true;
    }
}

pub fn find_convex_gate_ids3<const N_OUT: usize, R: RngCore>(
    circuit_num_wires: usize,
    circuit_gates: &[Gate],
    rng: &mut R,
) -> ([usize; N_OUT], usize) {
    let num_gates = circuit_gates.len();
    let num_wires = circuit_num_wires;
    let mut search_attempts = 0;
    loop {
        search_attempts += 1;

        let mut selected_gate_idx = [0; N_OUT];
        selected_gate_idx[0] = rng.random_range(0..num_gates);
        let mut selected_gate_ctr = 1;

        while selected_gate_ctr < N_OUT {
            let mut candidates: Vec<usize> = vec![];

            // Left-most gate, go right
            let mut path_connected_target_wires = PathConnectedWires::new(num_wires);
            let mut path_connected_control_wires = PathConnectedWires::new(num_wires);
            let mut selected_gates_seen = 1;
            if selected_gate_idx[0] != num_gates - 1 {
                for curr_idx in selected_gate_idx[0] + 1..num_gates {
                    if path_connected_target_wires.all_wires_hit()
                        || path_connected_control_wires.all_wires_hit()
                    {
                        break;
                    }
                    if curr_idx == selected_gate_idx[selected_gates_seen] {
                        // Next candidate
                        selected_gates_seen += 1;
                    } else {
                        // Not a selected gate
                        let curr_gate = circuit_gates[curr_idx];
                        let mut collides_with_prev_selected = false;
                        for i in 0..selected_gates_seen {
                            if curr_gate.collides_with(&circuit_gates[selected_gate_idx[i]]) {
                                collides_with_prev_selected = true;
                                break;
                            }
                        }
                        let [t, c1, c2] = curr_gate.wires;
                        let indirect_path_connected = path_connected_control_wires.wire_hit(t)
                            || path_connected_target_wires.wire_hit(c1)
                            || path_connected_target_wires.wire_hit(c2);

                        if collides_with_prev_selected || indirect_path_connected {
                            path_connected_target_wires.add_wire(t);
                            path_connected_control_wires.add_wire(c1);
                            path_connected_control_wires.add_wire(c2);

                            if !indirect_path_connected {
                                candidates.push(curr_idx);
                            }
                        }
                    }
                }
            }

            // Right-most gate, go left
            let mut path_connected_target_wires = PathConnectedWires::new(num_wires);
            let mut path_connected_control_wires = PathConnectedWires::new(num_wires);
            let mut selected_gates_seen = 1;
            if selected_gate_idx[selected_gate_ctr - 1] != 0 {
                for curr_idx in (0..=selected_gate_idx[selected_gate_ctr - 1] - 1).rev() {
                    if path_connected_target_wires.all_wires_hit()
                        || path_connected_control_wires.all_wires_hit()
                    {
                        break;
                    }
                    if selected_gates_seen < selected_gate_ctr
                        && curr_idx
                            == selected_gate_idx[selected_gate_ctr - 1 - selected_gates_seen]
                    {
                        // Next candidate
                        selected_gates_seen += 1;
                    } else {
                        // Not a selected gate
                        let curr_gate = circuit_gates[curr_idx];
                        let mut collides_with_prev_selected = false;
                        for i in 0..selected_gates_seen {
                            if curr_gate.collides_with(
                                &circuit_gates[selected_gate_idx[selected_gate_ctr - 1 - i]],
                            ) {
                                collides_with_prev_selected = true;
                                break;
                            }
                        }
                        let [t, c1, c2] = curr_gate.wires;
                        let indirect_path_connected = path_connected_control_wires.wire_hit(t)
                            || path_connected_target_wires.wire_hit(c1)
                            || path_connected_target_wires.wire_hit(c2);

                        if collides_with_prev_selected || indirect_path_connected {
                            path_connected_target_wires.add_wire(t);
                            path_connected_control_wires.add_wire(c1);
                            path_connected_control_wires.add_wire(c2);

                            if !indirect_path_connected {
                                candidates.push(curr_idx);
                            }
                        }
                    }
                }
            }

            // Break and choose new range if no candidates left to add
            if candidates.len() == 0 {
                break;
            }

            // Pick next gate at random among candidates
            let next_candidate = candidates.choose(rng).copied().unwrap();

            // Insert next_candidate into selected_gate_idx in order
            let mut insert_pos = selected_gate_ctr;
            while insert_pos > 0 && selected_gate_idx[insert_pos - 1] > next_candidate {
                selected_gate_idx[insert_pos] = selected_gate_idx[insert_pos - 1];
                insert_pos -= 1;
            }
            selected_gate_idx[insert_pos] = next_candidate;
            selected_gate_ctr += 1;
        }

        #[cfg(feature = "correctness")]
        assert!(is_convex(
            circuit_num_wires,
            circuit_gates,
            &selected_gate_idx
        ));

        return (selected_gate_idx, search_attempts);
    }
}

pub fn find_convex_gate_ids2<const N_OUT: usize, R: RngCore>(
    circuit: &Circuit,
    rng: &mut R,
) -> ([usize; N_OUT], usize) {
    let num_gates = circuit.gates.len();
    let num_wires = circuit.num_wires;
    let search_range = num_wires;
    let mut search_attempts = 0;
    loop {
        search_attempts += 1;
        let search_start = rng.random_range(0..num_gates - search_range + 1);

        let mut selected_gate_idx = [0; N_OUT];

        // Pick first gate
        selected_gate_idx[0] = rng.random_range(search_start..search_start + search_range);
        let mut selected_gate_ctr = 1;

        while selected_gate_ctr < N_OUT {
            // Find all gates in range that collide with >= 1 selected gate
            let candidates: Vec<usize> = (search_start..search_start + search_range)
                .filter(|&idx| {
                    for i in 0..selected_gate_ctr {
                        if idx == selected_gate_idx[i] {
                            return false;
                        }
                    }
                    for i in 0..selected_gate_ctr {
                        if circuit.gates[idx].collides_with(&circuit.gates[selected_gate_idx[i]]) {
                            return true;
                        }
                    }
                    false
                })
                .collect();
            // Break and choose new range if no candidates left to add
            if candidates.len() == 0 {
                break;
            }
            // Pick next gate at random among candidates
            selected_gate_idx[selected_gate_ctr] = candidates.choose(rng).copied().unwrap();
            selected_gate_ctr += 1;
        }
        if selected_gate_ctr != N_OUT {
            continue;
        }

        // selected_gate_idx is weakly-connected
        selected_gate_idx.sort_unstable();

        // Check that selected_gate_idx is convex
        let mut path_connected_target_wires = vec![false; num_wires];
        let mut path_connected_control_wires = vec![false; num_wires];
        let mut selected_gates_seen = 1;
        let mut not_convex = false;

        for idx in selected_gate_idx[0] + 1..=selected_gate_idx[N_OUT - 1] {
            if idx != selected_gate_idx[selected_gates_seen] {
                // Not a selected gate
                let curr_gate = circuit.gates[idx];
                let mut collides_with_prev_selected = false;
                for i in 0..selected_gates_seen {
                    if curr_gate.collides_with(&circuit.gates[selected_gate_idx[i]]) {
                        collides_with_prev_selected = true;
                        break;
                    }
                }
                let [t, c1, c2] = curr_gate.wires;
                if collides_with_prev_selected
                    || path_connected_control_wires[t]
                    || path_connected_target_wires[c1]
                    || path_connected_target_wires[c2]
                {
                    path_connected_target_wires[t] = true;
                    path_connected_control_wires[c1] = true;
                    path_connected_control_wires[c2] = true;
                }
            } else {
                // This is the next candidate
                let [t, c1, c2] = circuit.gates[selected_gate_idx[selected_gates_seen]].wires;
                if path_connected_control_wires[t]
                    || path_connected_target_wires[c1]
                    || path_connected_target_wires[c2]
                {
                    not_convex = true;
                    break;
                }

                selected_gates_seen += 1;
            }
        }

        if not_convex {
            continue;
        }

        return (selected_gate_idx, search_attempts);
    }
}

pub fn find_convex_gate_ids<const N_OUT: usize, R: RngCore>(
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

            // // pick gate 1 at random
            // selected_gate_idx[0] = rng.random_range(0..num_gates - N_OUT + 1);

            let gen_zero_idx = (0..num_gates - N_OUT + 1)
                .filter(|i| circuit.gates[*i].generation == 0)
                .collect::<Vec<_>>();
            dbg!(gen_zero_idx.len());
            selected_gate_idx[0] = gen_zero_idx.choose(rng).copied().unwrap();
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

pub fn permute_circuit<const N_OUT: usize>(
    circuit_num_wires: usize,
    circuit_gates: &mut [Gate],
    selected_gate_idx: &[usize; N_OUT],
) -> usize {
    let selected_gates = selected_gate_idx.map(|id| circuit_gates[id]);
    let mut to_before = vec![];
    let mut to_after = vec![];
    let mut path_connected_target_wires = vec![false; circuit_num_wires];
    let mut path_connected_control_wires = vec![false; circuit_num_wires];

    for j in 0..selected_gate_idx.len() - 1 {
        for i in selected_gate_idx[j] + 1..selected_gate_idx[j + 1] {
            let curr_gate = &circuit_gates[i];
            let curr_target = curr_gate.wires[0];
            let curr_control0 = curr_gate.wires[1];
            let curr_control1 = curr_gate.wires[2];

            let mut collides_with_prev_selected = false;
            for k in 0..=j {
                collides_with_prev_selected = collides_with_prev_selected
                    || circuit_gates[selected_gate_idx[k]].collides_with(curr_gate);
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
        circuit_gates[write_idx] = to_before[i];
        write_idx += 1;
    }
    let c_out_start = write_idx;
    for i in 0..N_OUT {
        circuit_gates[write_idx] = selected_gates[i];
        write_idx += 1;
    }
    for i in 0..to_after.len() {
        circuit_gates[write_idx] = to_after[i];
        write_idx += 1;
    }

    c_out_start
}

pub fn is_convex(
    circuit_num_wires: usize,
    circuit_gates: &[Gate],
    convex_gate_ids: &[usize],
) -> bool {
    let mut is_convex = true;

    let mut colliding_set = vec![];
    let mut path_colliding_targets = vec![false; circuit_num_wires];
    let mut path_colliding_controls = vec![false; circuit_num_wires];
    'outer: for i in convex_gate_ids[0]..*convex_gate_ids.last().unwrap() + 1 {
        if convex_gate_ids.contains(&i) {
            let selected_gate = circuit_gates[i];
            // check no collision with any gate in colliding_set
            for c_gate in colliding_set.iter() {
                if selected_gate.collides_with(c_gate) {
                    is_convex = false;
                    break 'outer;
                }
            }

            let [t, c0, c1] = circuit_gates[i].wires;
            path_colliding_targets[t] = true;
            path_colliding_controls[c0] = true;
            path_colliding_controls[c1] = true;
        } else {
            let [t, c0, c1] = circuit_gates[i].wires;
            if path_colliding_targets[c0]
                || path_colliding_targets[c1]
                || path_colliding_controls[t]
            {
                colliding_set.push(circuit_gates[i].clone());
                path_colliding_targets[t] = true;
                path_colliding_controls[c0] = true;
                path_colliding_controls[c1] = true;
            }
        }
    }

    is_convex
}

#[cfg(test)]
mod tests {
    use crate::{
        circuit::Circuit,
        local_mixing::{consts::N_OUT_KND, search::is_convex},
    };

    use super::find_convex_gate_ids3;

    #[test]
    fn test_find_convex() {
        let num_wires = 64;
        let num_gates = 10000;
        let mut rng = rand::rng();
        for i in 0..100000 {
            let circuit = Circuit::random_with_cf(
                num_wires,
                num_gates,
                crate::replacement::strategy::ControlFnChoice::All,
                &mut rng,
            );
            let (convex_gate_ids, _) =
                find_convex_gate_ids3::<N_OUT_KND, _>(circuit.num_wires, &circuit.gates, &mut rng);
            assert!(
                is_convex(circuit.num_wires, &circuit.gates, &convex_gate_ids),
                "failed at iteration {i}"
            );
        }
    }
}
