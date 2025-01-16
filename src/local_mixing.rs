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

    pub fn run_inflationary_step<R: RngCore>(&mut self, rng: &mut R) {
        let num_gates = self.current_circuit.gates.len();
        let num_wires = self.config.num_wires as usize;

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
                    dbg!("all wires hit!", i);
                    break;
                }
            }

            if !candidate_second_idxs.is_empty() {
                break;
            }
        }

        let gate_two_idx = candidate_second_idxs[rng.gen_range(0..candidate_second_idxs.len())];

        dbg!(gate_one_idx, gate_two_idx, candidate_second_idxs);

        let mut to_before = vec![];
        let mut to_after = vec![];

        {
            let between = self
                .current_circuit
                .gates
                .drain((gate_one_idx + 1)..gate_two_idx);
            for (i, gate) in between.enumerate() {
                if gate_one_successors.contains(&(gate_one_idx + 1 + i)) {
                    to_after.push(gate);
                } else {
                    to_before.push(gate);
                }
            }
        }

        self.current_circuit
            .gates
            .splice(gate_one_idx..gate_one_idx, to_before);
        self.current_circuit
            .gates
            .splice(gate_two_idx..gate_two_idx, to_after);
    }
}

#[cfg(test)]
mod tests {
    use rand::thread_rng;

    use super::*;

    #[test]
    fn test_inflationary_step() {
        let mut rng = thread_rng();
        let num_wires = 100;
        let num_gates = 10000000;
        let circuit = Circuit::random(num_wires, num_gates, &mut rng);
        // dbg!(circuit.clone());
        let config = LocalMixingConfig {
            original_circuit: circuit,
            num_wires,
            inflationary_steps: 1,
            kneading_steps: 1,
        };
        let mut local_mixing_job = LocalMixingJob::new(config);
        local_mixing_job.run_inflationary_step(&mut rng);
        // dbg!(local_mixing_job.current_circuit);
    }
}
