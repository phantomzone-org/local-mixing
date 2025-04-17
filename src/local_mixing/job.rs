use std::{cmp::min, error::Error, fs::File, io::BufReader, path::Path, time::Instant};

use rand::{rng, Rng, RngCore, SeedableRng};
use rand_chacha::ChaCha8Rng;
use rayon::{
    current_num_threads,
    iter::{IndexedParallelIterator, IntoParallelRefMutIterator, ParallelIterator},
    slice::ParallelSliceMut,
};
use serde::{Deserialize, Serialize};

use crate::{
    circuit::{circuit::check_ckt_equiv_inout_map, Circuit, Gate},
    compression::ct::CompressionTable,
    local_mixing::tracer::{SearchTraceFields, Tracer},
    replacement::{replace_ct::find_replacement, strategy::ControlFnChoice},
};

use super::{
    consts::{
        CORRECTNESS_CHECK_ITER, DEFAULT_NUM_GATES, DEFAULT_NUM_WIRES, N_IN, N_OUT_INF, N_OUT_KND,
    },
    search::{find_convex_gate_ids3, permute_circuit},
    tracer::init_logs,
    worker::Worker,
};

#[derive(Clone, Copy)]
pub enum LocalMixingStage {
    Inflationary,
    Kneading,
}

impl std::fmt::Display for LocalMixingStage {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let s = match self {
            LocalMixingStage::Inflationary => "Inflationary",
            LocalMixingStage::Kneading => "Kneading",
        };
        write!(f, "{}", s)
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct LocalMixingJob {
    /// Number of inflationary steps
    inflationary_stage_steps: usize,
    /// Number of kneading steps
    kneading_stage_steps: usize,
    /// Control function choice in replacement
    cf_choice: ControlFnChoice,
    /// Number of worker-threads for search
    search_threads: usize,
    /// Max number of samples allowed during replacement
    gate_sample_limit: usize,
    /// Save circuit after inflationary stage
    save_inflationary: bool,
    /// Path to compression table
    compression_table_path: String,
    /// Number of wires in auto-generated circuit
    #[serde(default)]
    num_wires: usize,
    /// Circuit
    #[serde(default, skip_serializing)]
    circuit: Circuit,
    /// Compression Table
    #[serde(default, skip_serializing)]
    ct: CompressionTable,
    #[serde(default, skip_serializing)]
    dir_path: String,
}

impl LocalMixingJob {
    pub fn load(dir_path: &str) -> Result<Self, Box<dyn Error>> {
        #[cfg(feature = "trace")]
        init_logs(dir_path)?;

        let config_path = format!("{}/config.json", dir_path);
        let input_circuit_path = format!("{}/input.json", dir_path);

        let mut job: Self = serde_json::from_reader(BufReader::new(File::open(&config_path)?))?;
        job.dir_path = dir_path.to_string();
        println!("-- Loaded job at {}", config_path);

        // Load input circuit
        if Path::new(&input_circuit_path).exists() {
            job.circuit = Circuit::load_from_json(input_circuit_path.clone());
            println!("-- Loaded circuit at {}", input_circuit_path);
        } else {
            println!("-- No input circuit found, generating");
            let mut rng = ChaCha8Rng::from_os_rng();
            job.circuit = Circuit::random_with_cf(
                if job.num_wires == 0 {
                    DEFAULT_NUM_WIRES
                } else {
                    job.num_wires
                },
                DEFAULT_NUM_GATES,
                job.cf_choice,
                &mut rng,
            );
            job.circuit.save_as_json(input_circuit_path.clone());
            println!("-- Saved random circuit into {}", input_circuit_path);
        }

        // Load compression table
        if Path::new(&job.compression_table_path).exists() {
            println!(
                "-- Loading compression table at {}",
                job.compression_table_path
            );
            job.ct = CompressionTable::from_file(&job.compression_table_path);
            assert!(job.cf_choice.cfs() == job.ct.cf_choice);
            println!("-- Loading compression table done");
        } else {
            println!(
                "-- No compression table found at {}, generating",
                job.compression_table_path
            );
            job.ct = CompressionTable::new(3, 9, job.cf_choice.cfs());
            job.ct.save_to_file(&job.compression_table_path);
            println!(
                "-- Save compression table into {}",
                job.compression_table_path
            );
        }

        Ok(job)
    }

    pub fn run(&mut self) {
        println!("-- Running");
        let start = Instant::now();
        if self.search_threads > 1 {
            self.run_multiple_threads();
        } else {
            self.run_one_thread();
        }
        let elapsed = Instant::now() - start;
        println!("-- Finished running in {:?}", elapsed);
    }

    pub fn run_one_thread<R: Rng + RngCore>(&mut self, rng: &mut R) {
        let mut tracer = Tracer::new(self.inflationary_stage_steps, self.kneading_stage_steps);

        println!("-- Inflationary stage");
        let mut inf_steps = 0;
        while inf_steps < self.inflationary_stage_steps {
            let success = run_step::<N_OUT_INF, N_IN, _>(
                &mut self.circuit,
                LocalMixingStage::Inflationary,
                inf_steps,
                &self.ct,
                self.gate_sample_limit,
                &mut tracer,
                rng,
            );
            if success {
                inf_steps += 1;
            }
        }
        println!("-- Inflationary stage: done");

        if self.save_inflationary && self.inflationary_stage_steps > 0 {
            let inflationary_path = format!("{}/inflationary.json", self.dir_path);
            self.circuit.save_as_json(inflationary_path.clone());
            println!("-- Saved inflationary stage to {}", inflationary_path);
        }
        self.circuit.reset_generations();

        println!("-- Kneading stage");
        let mut knd_steps = 0;
        while knd_steps < self.kneading_stage_steps {
            let success = run_step::<N_OUT_KND, N_IN, _>(
                &mut self.circuit,
                LocalMixingStage::Kneading,
                knd_steps,
                &self.ct,
                self.gate_sample_limit,
                &mut tracer,
                rng,
            );
            if success {
                knd_steps += 1;
            }
        }
        println!("-- Kneading stage: done");

        self.circuit
            .save_as_json(format!("{}/target.json", self.dir_path));

        #[cfg(feature = "trace")]
        {
            self.circuit
                .save_generation_data(format!("{}/generation.json", self.dir_path));
            log::info!(target: "trace", "Finished.");
            log::info!(target: "trace", "Inflationary stage successes: {}", tracer.step_statuses.inflationary_stage.success);
            log::info!(target: "trace", "Inflationary stage fails: {}", tracer.step_statuses.inflationary_stage.fail);
            log::info!(target: "trace", "Kneading stage successes: {}", tracer.step_statuses.kneading_stage.success);
            log::info!(target: "trace", "Kneading stage fails: {}", tracer.step_statuses.kneading_stage.fail);
            tracer.save_to_file(&self.dir_path);
        }
    }

    pub fn run_multiple_threads<R: Rng + RngCore>(&mut self, rng: &mut R) {
        let mut tracer = Tracer::new(self.inflationary_stage_steps, self.kneading_stage_steps);

        println!("-- Inflationary stage");
        let mut inf_steps = 0;
        while inf_steps < self.inflationary_stage_steps {
            let success = run_step::<N_OUT_INF, N_IN, _>(
                &mut self.circuit,
                LocalMixingStage::Inflationary,
                inf_steps,
                &self.ct,
                self.gate_sample_limit,
                &mut tracer,
                rng,
            );
            if success {
                inf_steps += 1;
            }
        }
        println!("-- Inflationary stage: done");

        if self.save_inflationary && self.inflationary_stage_steps > 0 {
            let inflationary_path = format!("{}/inflationary.json", self.dir_path);
            self.circuit.save_as_json(inflationary_path.clone());
            println!("-- Saved inflationary stage to {}", inflationary_path);
        }
        self.circuit.reset_generations();

        println!("-- Kneading stage");

        let num_search_workers = min(self.search_threads, current_num_threads());
        println!(
            "-- Initializing search workers. {} threads available",
            num_search_workers
        );

        let mut workers: Vec<Worker<ChaCha8Rng>> = (0..num_search_workers)
            .map(|worker_id| {
                Worker::new(
                    worker_id,
                    self.gate_sample_limit,
                    0,
                    self.kneading_stage_steps,
                )
            })
            .collect();

        let chunk_size = self.circuit.gates.len() / num_search_workers;
        self.circuit
            .gates
            .par_chunks_mut(chunk_size)
            .for_each(|s_circuit| {});

        let mut steps_completed = 0;
        while steps_completed < self.kneading_stage_steps {
            let phase1_circuits = self.circuit.split_into_chunks(num_search_workers, 0);
            self.circuit.gates = workers
                .par_iter_mut()
                .enumerate()
                .map(|(i, worker)| {
                    let ckt = phase1_circuits[i].clone();
                    let current_step = steps_completed + i;
                    worker.step(&LocalMixingStage::Kneading, current_step, ckt, &self.ct);

                    worker.get_current_circuit()
                })
                .flat_map(|ckt| ckt.gates)
                .collect();

            let phase2_circuits = self
                .circuit
                .split_into_chunks(num_search_workers, chunk_size / 2);

            self.circuit.gates = workers
                .par_iter_mut()
                .enumerate()
                .map(|(i, worker)| {
                    let ckt = phase2_circuits[i].clone();
                    let current_step = steps_completed + num_search_workers + i;
                    worker.step(&LocalMixingStage::Kneading, current_step, ckt, &self.ct);

                    worker.get_current_circuit()
                })
                .flat_map(|ckt| ckt.gates)
                .collect();

            steps_completed += 2 * num_search_workers;
        }
        println!("-- Kneading stage: done");

        self.circuit
            .save_as_json(format!("{}/target.json", self.dir_path));

        #[cfg(feature = "trace")]
        {
            self.circuit
                .save_generation_data(format!("{}/generation.json", self.dir_path));

            let mut all_workers = vec![inf_worker];
            all_workers.extend(workers);
            let tracer = Tracer::collect(all_workers.iter().map(|worker| worker.tracer()));
            tracer.save_to_file(&self.dir_path).unwrap();
            log::info!(target: "trace", "Finished.");
            log::info!(target: "trace", "Inflationary stage successes: {}", tracer.step_statuses.inflationary_stage.success);
            log::info!(target: "trace", "Inflationary stage fails: {}", tracer.step_statuses.inflationary_stage.fail);
            log::info!(target: "trace", "Kneading stage successes: {}", tracer.step_statuses.kneading_stage.success);
            log::info!(target: "trace", "Kneading stage fails: {}", tracer.step_statuses.kneading_stage.fail);
        }
    }
}

trait Growable {
    fn replace(self, start: usize, end: usize, gates: Vec<Gate>);
    fn gate_at_index(&self, index: usize) -> Gate;
    fn as_slice_ref<'a>(&'a self) -> &'a [Gate];
    fn as_slice_mut_ref<'a>(&'a mut self) -> &'a mut [Gate];
}

impl Growable for Vec<Gate> {
    fn replace(mut self, start: usize, end: usize, gates: Vec<Gate>) {
        self.splice(start..end, gates);
    }
    fn as_slice_ref<'a>(&'a self) -> &'a [Gate] {
        self.as_ref()
    }
    fn as_slice_mut_ref<'a>(&'a mut self) -> &'a mut [Gate] {
        self
    }
    fn gate_at_index(&self, index: usize) -> Gate {
        self[index]
    }
}
impl Growable for &mut [Gate] {
    fn replace(self, start: usize, end: usize, gates: Vec<Gate>) {
        self[start..end].copy_from_slice(&gates);
    }
    fn as_slice_ref<'a>(&'a self) -> &'a [Gate] {
        self
    }
    fn as_slice_mut_ref<'a>(&'a mut self) -> &'a mut [Gate] {
        self
    }
    fn gate_at_index(&self, index: usize) -> Gate {
        self[index]
    }
}

fn run_step<const N_OUT: usize, const N_IN: usize, R: Rng + RngCore>(
    circuit_num_wires: usize,
    circuit_gates: impl Growable,
    stage: LocalMixingStage,
    current_step: usize,
    ct: &CompressionTable,
    gate_sample_limit: usize,
    tracer: &mut Tracer,
    rng: &mut R,
) -> bool {
    #[cfg(feature = "trace")]
    let start_time = Instant::now();

    let (selected_gate_idx, _n_search_attempts) =
        find_convex_gate_ids3::<N_OUT, _>(circuit_num_wires, circuit_gates.as_slice_ref(), rng);

    let c_out = selected_gate_idx
        .iter()
        .map(|i| circuit_gates.gate_at_index(*i))
        .collect::<Vec<_>>();

    #[cfg(feature = "trace")]
    let repl_start = Instant::now();

    let replacement_res =
        find_replacement(&c_out, circuit_num_wires, N_IN, gate_sample_limit, ct, rng);

    #[cfg(feature = "trace")]
    let _replacement_time = Instant::now() - repl_start;

    if let Some((c_in, _replacement_fields)) = replacement_res {
        #[cfg(feature = "trace")]
        {
            if current_step % 10000 == 0 {
                tracer.add_replacement_sample(stage, c_out, c_in.clone());
            }
        }

        // generate input output table
        #[cfg(feature = "correctness")]
        let inout: Vec<(Vec<bool>, Vec<bool>)> = (0..CORRECTNESS_CHECK_ITER)
            .map(|_| {
                let input: Vec<bool> = (0..circuit_num_wires)
                    .map(|_| rng.random_bool(0.5))
                    .collect();
                // TODO: circuit.evaluate(&input)
                let output = vec![];
                (input, output)
            })
            .collect();

        let c_out_start = permute_circuit(
            circuit_num_wires,
            circuit_gates.as_slice_mut_ref(),
            &selected_gate_idx,
        );
        circuit_gates.replace(c_out_start, c_out_start + N_OUT, c_in);

        let _final_end_time = Instant::now() - start_time;

        #[cfg(feature = "correctness")]
        {
            if check_ckt_equiv_inout_map(&inout, circuit_gates.as_slice_ref()) == false {
                let error_str = format!("{}, step={}, Obfuscated circuit is functionally not equivalent to original input circuit", stage,  current_step);
                log::error!(target: "trace", "{error_str}");
                panic!("{error_str}");
            }
        }

        #[cfg(feature = "trace")]
        {
            let search_fields = SearchTraceFields {
                n_gates: circuit_gates.as_slice_ref().len(),
                n_search_attempts: _n_search_attempts,
                time: _final_end_time,
            };
            tracer.add_entry(
                &stage,
                search_fields.clone(),
                _replacement_fields.clone(),
                _replacement_time,
            );
            tracer.inc_success(&stage);

            log::info!(target: "trace", "{}", format!("{}, step={}, SUCCESS: n_gates = {}, n_circuits_sampled = {}, n_search_attempts = {}, time = {:?}", 
                    stage, current_step, search_fields.n_gates, _replacement_fields.num_circuits_sampled, search_fields.n_search_attempts, search_fields.time));
        }

        return true;
    } else {
        #[cfg(feature = "trace")]
        {
            log::warn!(target: "trace", "{}, step = {}, FAILED: failed to find replacement for {:?}",
                        stage,  current_step, c_out);
            tracer.inc_fail(stage);
        }

        return false;
    }
}
