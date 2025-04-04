use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

use crate::{
    circuit::Circuit, compression::ct::CompressionTable, replacement::replace_ct::find_replacement,
};

use super::{
    consts::{N_IN, N_OUT_INF, N_OUT_KND},
    job::LocalMixingStage,
    search::{find_convex_gate_ids, permute_circuit},
};

pub struct Worker<R: Rng> {
    id: usize,
    ct: CompressionTable,
    rng: R,
    circuit: Circuit,
}

impl Worker<ChaCha8Rng> {
    pub fn new(id: usize, ct: CompressionTable) -> Self {
        let rng = ChaCha8Rng::from_os_rng();
        Self {
            id,
            ct,
            rng,
            circuit: Circuit::default(),
        }
    }

    pub fn get_current_circuit(&self) -> Circuit {
        self.circuit.clone()
    }

    pub fn run_task(&mut self, stage: LocalMixingStage, num_steps: usize, input_circuit: Circuit) {
        self.circuit = input_circuit;

        match stage {
            LocalMixingStage::Inflationary => {
                self.run_stage_specific_task::<N_OUT_INF>(num_steps);
            }
            LocalMixingStage::Kneading => {
                self.run_stage_specific_task::<N_OUT_KND>(num_steps);
            }
        }
    }

    fn run_stage_specific_task<const N_OUT: usize>(&mut self, num_steps: usize) {
        let mut current_step = 1;

        while current_step <= num_steps {
            let (selected_gate_idx, _) =
                find_convex_gate_ids::<N_OUT, _>(&self.circuit, &mut self.rng);
            let selected_gates = selected_gate_idx
                .iter()
                .map(|i| self.circuit.gates[*i])
                .collect::<Vec<_>>();

            let replacement_res = find_replacement(
                &selected_gates,
                self.circuit.num_wires,
                N_IN,
                &mut self.ct,
                &mut self.rng,
            );

            if let Some((c_in, _)) = replacement_res {
                let c_out_start = permute_circuit(&mut self.circuit, &selected_gate_idx);
                self.circuit
                    .gates
                    .splice(c_out_start..c_out_start + N_OUT, c_in);

                current_step += 1;
            } else {
                println!("Failed");
            }
        }
    }
}
