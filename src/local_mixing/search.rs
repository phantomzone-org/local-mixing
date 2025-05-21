use std::collections::HashSet;

use crate::circuit::Gate;
use rand::{seq::IndexedRandom, Rng, RngCore};

pub struct PathConnectedWires {
    wires: Vec<bool>,
    count: usize,
}

impl PathConnectedWires {
    pub fn new(num_wires: usize) -> Self {
        Self {
            wires: vec![false; num_wires],
            count: 0,
        }
    }

    pub fn all_wires_hit(&self) -> bool {
        self.count == self.wires.len()
    }

    pub fn wire_hit(&self, wire: usize) -> bool {
        self.wires[wire]
    }

    pub fn add_wire(&mut self, wire: usize) {
        if !self.wires[wire] {
            self.count += 1;
        }
        self.wires[wire] = true;
    }

    pub fn count(&self) -> usize {
        self.count
    }
}

pub fn random_convex<R: RngCore>(
    set_size: usize,
    max_wires: usize,
    circuit_num_wires: usize,
    circuit_gates: &[Gate],
    rng: &mut R,
) -> (Vec<usize>, usize) {
    let mut search_attempts = 0;
    loop {
        search_attempts += 1;
        let mut selected_gate_idx = Vec::with_capacity(set_size);

        // First gate
        let first_gate_idx = rng.random_range(0..circuit_gates.len());
        let mut selected_distinct = PathConnectedWires::new(circuit_num_wires);
        circuit_gates[first_gate_idx]
            .wires
            .iter()
            .for_each(|&w| selected_distinct.add_wire(w));

        selected_gate_idx.push(first_gate_idx);

        loop {
            if selected_gate_idx.len() == set_size {
                return (selected_gate_idx, search_attempts);
            }

            let mut candidates = vec![];
            let mut needs_both_passes = vec![];

            // To the right of selected_gate_idx
            let mut selected_targets = PathConnectedWires::new(circuit_num_wires);
            let mut selected_controls = PathConnectedWires::new(circuit_num_wires);
            let mut path_targets = PathConnectedWires::new(circuit_num_wires);
            let mut path_controls = PathConnectedWires::new(circuit_num_wires);
            let mut selected_gates_seen = 0;

            for i in selected_gate_idx[0]..circuit_gates.len() {
                if path_targets.all_wires_hit() || path_controls.all_wires_hit() {
                    break;
                }

                let [t, c1, c2] = circuit_gates[i].wires;
                if selected_gates_seen < selected_gate_idx.len()
                    && i == selected_gate_idx[selected_gates_seen]
                {
                    selected_gates_seen += 1;
                    selected_targets.add_wire(t);
                    selected_controls.add_wire(c1);
                    selected_controls.add_wire(c2);
                } else {
                    let collides = selected_controls.wire_hit(t)
                        || selected_targets.wire_hit(c1)
                        || selected_targets.wire_hit(c2);
                    let indirect_path_connected = path_controls.wire_hit(t)
                        || path_targets.wire_hit(c1)
                        || path_targets.wire_hit(c2);
                    if collides || indirect_path_connected {
                        path_targets.add_wire(t);
                        path_controls.add_wire(c1);
                        path_controls.add_wire(c2);
                    }

                    let enough_wires_left = circuit_gates[i]
                        .wires
                        .iter()
                        .filter(|&w| !selected_distinct.wire_hit(*w))
                        .count()
                        + selected_distinct.count()
                        <= max_wires;

                    if !indirect_path_connected && enough_wires_left {
                        if i < selected_gate_idx[selected_gate_idx.len() - 1] {
                            needs_both_passes.push(i);
                        } else {
                            candidates.push(i);
                        }
                    }
                }
            }

            // To the left of selected_gate_idx
            let mut selected_targets = PathConnectedWires::new(circuit_num_wires);
            let mut selected_controls = PathConnectedWires::new(circuit_num_wires);
            let mut path_targets = PathConnectedWires::new(circuit_num_wires);
            let mut path_controls = PathConnectedWires::new(circuit_num_wires);
            let mut selected_gates_seen = 0;

            for i in (0..selected_gate_idx[selected_gate_idx.len() - 1] + 1).rev() {
                if path_targets.all_wires_hit() || path_controls.all_wires_hit() {
                    break;
                }

                let [t, c1, c2] = circuit_gates[i].wires;
                if selected_gates_seen < selected_gate_idx.len()
                    && i == selected_gate_idx[selected_gate_idx.len() - 1 - selected_gates_seen]
                {
                    selected_gates_seen += 1;
                    selected_targets.add_wire(t);
                    selected_controls.add_wire(c1);
                    selected_controls.add_wire(c2);
                } else {
                    let collides = selected_controls.wire_hit(t)
                        || selected_targets.wire_hit(c1)
                        || selected_targets.wire_hit(c2);
                    let indirect_path_connected = path_controls.wire_hit(t)
                        || path_targets.wire_hit(c1)
                        || path_targets.wire_hit(c2);
                    if collides || indirect_path_connected {
                        path_targets.add_wire(t);
                        path_controls.add_wire(c1);
                        path_controls.add_wire(c2);
                    }

                    let enough_wires_left = circuit_gates[i]
                        .wires
                        .iter()
                        .filter(|&w| !selected_distinct.wire_hit(*w))
                        .count()
                        + selected_distinct.count()
                        <= max_wires;

                    if !indirect_path_connected && enough_wires_left {
                        if i < selected_gate_idx[0] || needs_both_passes.binary_search(&i).is_ok() {
                            candidates.push(i);
                        }
                    }
                }
            }

            if candidates.len() == 0 {
                break;
            }

            let next_candidate = candidates.choose(rng).copied().unwrap();
            circuit_gates[next_candidate]
                .wires
                .iter()
                .for_each(|&w| selected_distinct.add_wire(w));

            match selected_gate_idx.binary_search(&next_candidate) {
                Ok(_) => panic!(),
                Err(p) => selected_gate_idx.insert(p, next_candidate),
            };
        }
    }
}

pub fn find_convex_gate_ids3<R: RngCore>(
    set_size: usize,
    max_wires: usize,
    circuit_num_wires: usize,
    circuit_gates: &[Gate],
    rng: &mut R,
) -> (Vec<usize>, usize) {
    let num_gates = circuit_gates.len();
    let num_wires = circuit_num_wires;
    let mut search_attempts = 0;
    loop {
        search_attempts += 1;

        let mut selected_gate_idx = vec![0; set_size];
        selected_gate_idx[0] = rng.random_range(0..num_gates);
        let mut selected_gate_ctr = 1;
        let mut curr_wires = HashSet::new();
        curr_wires.extend(circuit_gates[selected_gate_idx[0]].wires);

        while selected_gate_ctr < set_size {
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

                            let num_new_wires = curr_gate
                                .wires
                                .iter()
                                .filter(|&w| !curr_wires.contains(w))
                                .count();

                            if !indirect_path_connected
                                && curr_wires.len() + num_new_wires <= max_wires
                            {
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

                            let num_new_wires = curr_gate
                                .wires
                                .iter()
                                .filter(|&w| !curr_wires.contains(w))
                                .count();

                            if !indirect_path_connected
                                && curr_wires.len() + num_new_wires <= max_wires
                            {
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

            curr_wires.extend(circuit_gates[next_candidate].wires);
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

pub fn permute_circuit(
    circuit_num_wires: usize,
    circuit_gates: &mut [Gate],
    selected_gate_idx: &Vec<usize>,
) -> usize {
    let selected_gates: Vec<_> = selected_gate_idx
        .iter()
        .map(|&id| circuit_gates[id])
        .collect();
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
    for i in 0..selected_gate_idx.len() {
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
        circuit::{cf::GateLibrary, Circuit},
        local_mixing::search::{is_convex, random_convex},
    };

    #[test]
    fn test_find_convex() {
        let num_wires = 64;
        let num_gates = 1000;
        let mut rng = rand::rng();
        for i in 0..100000 {
            let circuit = Circuit::random_with_cf(num_wires, num_gates, GateLibrary::All, &mut rng);
            let (convex_gate_ids, _) =
                random_convex(4, 6, circuit.num_wires, &circuit.gates, &mut rng);
            let gates: Vec<_> = convex_gate_ids.iter().map(|&i| circuit.gates[i]).collect();
            dbg!(&gates);
            if !is_convex(circuit.num_wires, &circuit.gates, &convex_gate_ids) {
                dbg!(&circuit.gates);
                dbg!(&convex_gate_ids);
            }
            assert!(
                is_convex(circuit.num_wires, &circuit.gates, &convex_gate_ids),
                "failed at iteration {i}"
            );
        }
    }
}
