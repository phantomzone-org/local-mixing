use std::time::Instant;

use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

use crate::{
    circuit::{circuit::check_equiv_probabilistic, Circuit},
    compression::ct::CompressionTable,
    replacement::replace_ct::find_replacement,
};

#[cfg(feature = "correctness")]
use crate::local_mixing::consts::CORRECTNESS_CHECK_ITER;

use super::{
    consts::{N_IN, N_OUT_INF, N_OUT_KND},
    job::LocalMixingStage,
    search::{find_convex_gate_ids3, permute_circuit},
    tracer::{SearchTraceFields, Tracer},
};

pub struct Worker<R: Rng> {
    id: usize,
    rng: R,
    gate_sample_limit: usize,
    circuit: Circuit,
    #[cfg(feature = "trace")]
    tracer: Tracer,
    #[cfg(feature = "correctness")]
    original_circuit: Circuit,
}

impl Worker<ChaCha8Rng> {
    pub fn new(
        id: usize,
        gate_sample_limit: usize,
        inf_capacity: usize,
        knd_capacity: usize,
    ) -> Self {
        let rng = ChaCha8Rng::from_os_rng();
        Self {
            id,
            rng,
            gate_sample_limit,
            circuit: Circuit::default(),
            #[cfg(feature = "trace")]
            tracer: Tracer::new(inf_capacity, knd_capacity),
            #[cfg(feature = "correctness")]
            original_circuit: Circuit::default(),
        }
    }

    pub fn get_current_circuit(&self) -> Circuit {
        self.circuit.clone()
    }

    pub fn run_task(
        &mut self,
        stage: &LocalMixingStage,
        num_steps: usize,
        input_circuit: Circuit,
        ct: &CompressionTable,
    ) {
        #[cfg(feature = "correctness")]
        {
            self.original_circuit = input_circuit.clone();
        }

        self.circuit = input_circuit;

        match stage {
            LocalMixingStage::Inflationary => {
                self.run_stage_specific_task::<N_OUT_INF>(stage, num_steps, ct);
            }
            LocalMixingStage::Kneading => {
                self.run_stage_specific_task::<N_OUT_KND>(stage, num_steps, ct);
            }
        }
    }

    fn run_stage_specific_task<const N_OUT: usize>(
        &mut self,
        stage: &LocalMixingStage,
        num_steps: usize,
        ct: &CompressionTable,
    ) {
        let mut current_step = 1;

        while current_step <= num_steps {
            #[cfg(feature = "trace")]
            let start_time = Instant::now();

            let (selected_gate_idx, _max_candidate_dist) =
                find_convex_gate_ids3::<N_OUT, _>(&self.circuit, &mut self.rng);
            let selected_gates = selected_gate_idx
                .iter()
                .map(|i| self.circuit.gates[*i])
                .collect::<Vec<_>>();

            #[cfg(feature = "trace")]
            let repl_start = Instant::now();

            let replacement_res = find_replacement(
                &selected_gates,
                self.circuit.num_wires,
                N_IN,
                self.gate_sample_limit,
                ct,
                &mut self.rng,
            );

            #[cfg(feature = "trace")]
            let replacement_time = Instant::now() - repl_start;

            if let Some((c_in, _replacement_fields)) = replacement_res {
                let c_out_start = permute_circuit(&mut self.circuit, &selected_gate_idx);
                self.circuit
                    .gates
                    .splice(c_out_start..c_out_start + N_OUT, c_in);

                #[cfg(feature = "trace")]
                {
                    let search_fields = SearchTraceFields {
                        n_gates: self.circuit.gates.len(),
                        max_candidate_dist: _max_candidate_dist,
                        time: Instant::now() - start_time,
                    };
                    self.tracer.add_entry(
                        stage,
                        search_fields.clone(),
                        _replacement_fields.clone(),
                        replacement_time,
                    );

                    log::info!(target: "trace", "{}", format!("{}, worker = {}, step={}, SUCCESS: n_gates = {}, n_circuits_sampled = {}, max_candidate_dist = {}, time = {:?}", 
                    stage, self.id, current_step, search_fields.n_gates, _replacement_fields.num_circuits_sampled, search_fields.max_candidate_dist, search_fields.time));
                }

                #[cfg(feature = "correctness")]
                {
                    if check_equiv_probabilistic(
                        self.circuit.num_wires,
                        &self.original_circuit.gates,
                        &self.circuit.gates,
                        CORRECTNESS_CHECK_ITER,
                        &mut self.rng,
                    )
                    .is_err()
                    {
                        let error_str = format!("{}, worker = {}, step={}, Obfuscated circuit is functionally not equivalent to original input circuit", LocalMixingStage::Inflationary, self.id, current_step);
                        log::error!(target: "trace", "{error_str}");
                        panic!("{error_str}");
                    }
                }

                current_step += 1;
            } else {
                #[cfg(feature = "trace")]
                log::warn!(target: "trace", "{}, worker = {}, step = {}, FAILED: failed to find replacement for {:?}",
                stage, self.id, current_step, selected_gates);
            }
        }
    }

    #[cfg(feature = "trace")]
    pub fn tracer(&self) -> Tracer {
        self.tracer.clone()
    }

    #[cfg(feature = "trace")]
    pub fn save_trace(&self, dir_path: &str) {
        self.tracer.save_to_file(dir_path).unwrap();
    }
}
