use std::{
    cmp::min,
    error::Error,
    fs::File,
    io::BufReader,
    path::Path,
    sync::atomic::{AtomicUsize, Ordering},
    time::Instant,
};

use rand::{Rng, RngCore, SeedableRng};
use rand_chacha::ChaCha8Rng;
use rayon::{
    current_num_threads,
    iter::{IndexedParallelIterator, IntoParallelRefMutIterator, ParallelIterator},
    slice::ParallelSliceMut,
};
use serde::{Deserialize, Serialize};

use crate::{
    circuit::{
        circuit::{check_ckt_equiv_inout_map, evaluate},
        Circuit, Gate,
    },
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
        let mut rng = ChaCha8Rng::from_os_rng();
        let start = Instant::now();
        if self.search_threads > 1 {
            self.run_multiple_threads(&mut rng);
        } else {
            self.run_one_thread(&mut rng);
        }
        let elapsed = Instant::now() - start;
        println!("-- Finished running in {:?}", elapsed);
    }

    pub fn run_one_thread<R: Rng + RngCore>(&mut self, rng: &mut R) {
        let mut tracer = Tracer::new(self.inflationary_stage_steps, self.kneading_stage_steps);

        println!("-- Inflationary stage");
        let mut inf_steps = 0;
        let mut inf_fails = 0;
        while inf_steps < self.inflationary_stage_steps {
            let success = run_step::<N_OUT_INF, N_IN, _, _>(
                self.circuit.num_wires,
                &mut self.circuit.gates,
                LocalMixingStage::Inflationary,
                inf_steps,
                &self.ct,
                self.gate_sample_limit,
                &mut tracer,
                rng,
            );
            if success {
                inf_steps += 1;
            } else {
                inf_fails += 1;
            }
        }
        println!("-- Inflationary stage: done");

        if self.save_inflationary && self.inflationary_stage_steps > 0 {
            let inflationary_path = format!("{}/inflationary.json", self.dir_path);
            self.circuit.save_as_json(inflationary_path.clone());
            self.circuit
                .save_generation_data(format!("{}/inflationary.generations.json", self.dir_path));
            println!("-- Saved inflationary stage to {}", inflationary_path);
        }
        self.circuit.reset_generations();

        println!("-- Kneading stage");
        let mut knd_steps = 0;
        let mut knd_fails = 0;
        while knd_steps < self.kneading_stage_steps {
            let success = run_step::<N_OUT_KND, N_IN, _, _>(
                self.circuit.num_wires,
                &mut self.circuit.gates[..],
                LocalMixingStage::Kneading,
                knd_steps,
                &self.ct,
                self.gate_sample_limit,
                &mut tracer,
                rng,
            );
            if success {
                knd_steps += 1;
            } else {
                knd_fails += 1;
            }
        }
        println!("-- Kneading stage: done");

        self.circuit
            .save_as_json(format!("{}/target.json", self.dir_path));

        #[cfg(feature = "trace")]
        {
            self.circuit
                .save_generation_data(format!("{}/generation.json", self.dir_path));
            tracer
                .save_to_file(&self.dir_path)
                .expect("Failed to save trace");
            log::info!(target: "trace", "Finished.");
            log::info!(target: "trace", "Inflationary stage fails: {}", inf_fails);
            log::info!(target: "trace", "Kneading stage fails: {}", knd_fails);
        }
    }

    pub fn run_multiple_threads<R: Send + Sync + RngCore + SeedableRng>(&mut self, rng: &mut R) {
        let mut inf_tracer = Tracer::new(self.inflationary_stage_steps, 0);

        println!("-- Inflationary stage");
        let mut inf_steps = 0;
        let mut inf_fails = 0;
        while inf_steps < self.inflationary_stage_steps {
            let success = run_step::<N_OUT_INF, N_IN, _, _>(
                self.circuit.num_wires,
                &mut self.circuit.gates,
                LocalMixingStage::Inflationary,
                inf_steps,
                &self.ct,
                self.gate_sample_limit,
                &mut inf_tracer,
                rng,
            );
            if success {
                inf_steps += 1;
            } else {
                inf_fails += 1;
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
        println!("-- Using {} search threads", num_search_workers);

        let num_gates = self.circuit.gates.len();
        let chunk_size = num_gates / num_search_workers;
        let mut knd_tracers = vec![Tracer::new(0, self.kneading_stage_steps); num_search_workers];
        let mut rngs: Vec<_> = (0..num_search_workers)
            .map(|_| ChaCha8Rng::from_os_rng())
            .collect();

        let mut knd_steps = 0;
        let knd_fails = AtomicUsize::new(0);
        while knd_steps < self.kneading_stage_steps {
            // Phase 1: even chunks
            self.circuit
                .gates
                .par_chunks_mut(chunk_size)
                .zip(knd_tracers.par_iter_mut())
                .zip(rngs.par_iter_mut())
                .for_each(|((chunk, tracer), rng)| loop {
                    let success = run_step::<N_OUT_KND, N_IN, _, _>(
                        self.circuit.num_wires,
                        chunk,
                        LocalMixingStage::Kneading,
                        knd_steps,
                        &self.ct,
                        self.gate_sample_limit,
                        tracer,
                        rng,
                    );
                    if success {
                        break;
                    }
                    knd_fails.fetch_add(1, Ordering::Relaxed);
                });

            // Phase 2: |1st chunk| = chunk_size / 2, |last chunk| = chunk_size * 3 / 2
            let mut chunks = vec![];
            let (first, rest) = self.circuit.gates.split_at_mut(chunk_size / 2);
            let (middle, last) = rest.split_at_mut(num_gates - chunk_size * 2);
            chunks.push(first);
            middle
                .chunks_mut(chunk_size)
                .for_each(|chunk| chunks.push(chunk));
            chunks.push(last);

            chunks
                .par_iter_mut()
                .zip(knd_tracers.par_iter_mut())
                .zip(rngs.par_iter_mut())
                .for_each(|((chunk, tracer), rng)| loop {
                    let success = run_step::<N_OUT_KND, N_IN, _, _>(
                        self.circuit.num_wires,
                        chunk,
                        LocalMixingStage::Kneading,
                        knd_steps,
                        &self.ct,
                        self.gate_sample_limit,
                        tracer,
                        rng,
                    );
                    if success {
                        break;
                    }
                    knd_fails.fetch_add(1, Ordering::Relaxed);
                });

            // Phase 3: |1st chunk| = chunk_size * 3 / 2, |last chunk| = chunk_size / 2
            let mut chunks = vec![];
            let (first, rest) = self.circuit.gates.split_at_mut(chunk_size * 3 / 2);
            let (middle, last) = rest.split_at_mut(num_gates - chunk_size * 2);
            chunks.push(first);
            middle
                .chunks_mut(chunk_size)
                .for_each(|chunk| chunks.push(chunk));
            chunks.push(last);

            chunks
                .par_iter_mut()
                .zip(knd_tracers.par_iter_mut())
                .zip(rngs.par_iter_mut())
                .for_each(|((chunk, tracer), rng)| loop {
                    let success = run_step::<N_OUT_KND, N_IN, _, _>(
                        self.circuit.num_wires,
                        chunk,
                        LocalMixingStage::Kneading,
                        knd_steps,
                        &self.ct,
                        self.gate_sample_limit,
                        tracer,
                        rng,
                    );
                    if success {
                        break;
                    }
                    knd_fails.fetch_add(1, Ordering::Relaxed);
                });

            knd_steps += 3 * num_search_workers;
        }

        println!("-- Kneading stage: done");

        self.circuit
            .save_as_json(format!("{}/target.json", self.dir_path));

        #[cfg(feature = "trace")]
        {
            self.circuit
                .save_generation_data(format!("{}/generation.json", self.dir_path));

            let mut all_tracers = vec![inf_tracer];
            all_tracers.extend(knd_tracers);
            let tracer = Tracer::collect(all_tracers);
            tracer
                .save_to_file(&self.dir_path)
                .expect("Failed to save trace");
            log::info!(target: "trace", "Finished.");
            log::info!(target: "trace", "Inflationary stage fails: {}", inf_fails);
            log::info!(target: "trace", "Kneading stage fails: {}", knd_fails.load(Ordering::Relaxed));
        }
    }
}

trait Growable {
    fn replace(&mut self, start: usize, end: usize, gates: Vec<Gate>);
    fn gate_at_index(&self, index: usize) -> Gate;
    fn as_slice_ref(&self) -> &[Gate];
    fn as_slice_mut_ref(&mut self) -> &mut [Gate];
}

impl Growable for Vec<Gate> {
    fn replace(&mut self, start: usize, end: usize, gates: Vec<Gate>) {
        self.splice(start..end, gates);
    }
    fn as_slice_ref(&self) -> &[Gate] {
        self.as_ref()
    }
    fn as_slice_mut_ref(&mut self) -> &mut [Gate] {
        self
    }
    fn gate_at_index(&self, index: usize) -> Gate {
        self[index]
    }
}

impl Growable for [Gate] {
    fn replace(&mut self, start: usize, end: usize, gates: Vec<Gate>) {
        self[start..end].copy_from_slice(&gates);
    }
    fn as_slice_ref(&self) -> &[Gate] {
        self
    }
    fn as_slice_mut_ref(&mut self) -> &mut [Gate] {
        self
    }
    fn gate_at_index(&self, index: usize) -> Gate {
        self[index]
    }
}

impl Growable for &mut [Gate] {
    fn replace(&mut self, start: usize, end: usize, gates: Vec<Gate>) {
        self[start..end].copy_from_slice(&gates);
    }
    fn as_slice_ref(&self) -> &[Gate] {
        self
    }
    fn as_slice_mut_ref(&mut self) -> &mut [Gate] {
        self
    }
    fn gate_at_index(&self, index: usize) -> Gate {
        self[index]
    }
}

fn run_step<const N_OUT: usize, const N_IN: usize, G: Growable + ?Sized, R: Rng + RngCore>(
    circuit_num_wires: usize,
    circuit_gates: &mut G,
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
                let output = evaluate(circuit_gates.as_slice_ref(), &input);
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
                stage,
                search_fields.clone(),
                _replacement_fields.clone(),
                _replacement_time,
            );

            log::info!(target: "trace", "{}", format!("{}, step={}, SUCCESS: n_gates = {}, n_circuits_sampled = {}, n_search_attempts = {}, time = {:?}", 
                    stage, current_step, search_fields.n_gates, _replacement_fields.num_circuits_sampled, search_fields.n_search_attempts, search_fields.time));
        }

        return true;
    } else {
        #[cfg(feature = "trace")]
        {
            log::warn!(target: "trace", "{}, step = {}, FAILED: failed to find replacement for {:?}",
                        stage,  current_step, c_out);
        }

        return false;
    }
}
