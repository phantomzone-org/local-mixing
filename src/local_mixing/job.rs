use std::{cmp::min, error::Error, fs::File, io::BufReader, path::Path, time::Instant};

use rand::{RngCore, SeedableRng};
use rand_chacha::ChaCha8Rng;
use rayon::{
    current_num_threads,
    iter::{IndexedParallelIterator, IntoParallelRefMutIterator, ParallelIterator},
};
use serde::{Deserialize, Serialize};
use super::{
    consts::{N_OUT_INF, N_OUT_KND},
    search::{find_convex_gate_ids3, find_convex_gate_ids_max_spread, find_convex_gate_ids_max_spread_overall, permute_circuit},
};
use crate::{
    circuit::{
        cf::GateLibrary, Circuit, Gate
    },
    compression::ct::CompressionTable,
    local_mixing::{
        consts::{EPOCH_SIZE, N_IN_INF, N_IN_KND},
        tracer::Tracer,
    }, replacement::{find_replacement_random_sample, replace_ct::find_replacement_with_ct},
};

#[cfg(feature = "trace")]
use super::tracer::init_logs;
#[cfg(feature = "trace")]
use crate::{
    circuit::circuit::{circuit_min_generation, GateData},
    local_mixing::tracer::{ReplacementStatus, ReplacementTraceFields, SearchTraceFields}
};

#[cfg(feature = "correctness")]
use super::consts::CORRECTNESS_CHECK_ITER;
#[cfg(feature = "correctness")]
use crate::circuit::circuit::{check_ckt_equiv_inout_map, evaluate};
#[cfg(feature = "correctness")]
use rand::Rng;

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
    gate_library: GateLibrary,
    /// Number of worker-threads for search
    search_threads: usize,
    /// Max number of samples allowed during replacement
    max_circuit_samples: usize,
    /// Save circuit after inflationary stage
    save_inflationary: bool,
    /// Path to compression table
    compression_table_path: String,
    /// Number of wires in auto-generated circuit
    #[serde(default, skip_serializing)]
    num_wires: usize,
    #[serde(default, skip_serializing)]
    original_num_gates: usize,
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
            if job.num_wires == 0 {
                return Err("num_wires must be set in config.json or input.json must exist".into());
            }
            if job.original_num_gates == 0 {
                return Err("original_num_gates must be set in config.json or input.json must exist".into());
            }
            let mut rng = ChaCha8Rng::from_os_rng();
            job.circuit = Circuit::random_with_cf(
                job.num_wires,
                job.original_num_gates,
                job.gate_library,
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
            assert!(job.gate_library == job.ct.gate_library);
            println!("-- Loading compression table done");
        } else {
            println!(
                "-- No compression table found at {}, generating",
                job.compression_table_path
            );
            job.ct = CompressionTable::new(3, 9, job.gate_library);
            job.ct.save_to_file(&job.compression_table_path);
            println!(
                "-- Save compression table into {}",
                job.compression_table_path
            );
        }

        Ok(job)
    }

    pub fn save_config_json(&self, path: &str) -> Result<(), Box<dyn Error>> {
         std::fs::write(path, serde_json::to_vec_pretty(&self)?)?;
         Ok(())
    }

    pub fn run(&mut self) {
        println!("-- Running");
        let mut rng = ChaCha8Rng::from_os_rng();
        let start = Instant::now();
        if self.search_threads > 1 {
            self.run_multiple_threads(&mut rng);
        } else {
            self.run_single_threaded(&mut rng);
        }
        let elapsed = Instant::now() - start;
        println!("-- Finished running in {:?}", elapsed);
    }

    pub fn run_single_threaded<R: Send + Sync + RngCore + SeedableRng>(&mut self, rng: &mut R) {
        let mut tracer = Tracer::new(self.inflationary_stage_steps, self.kneading_stage_steps);

        println!("-- Inflationary stage");
        let mut inf_steps = 0;
        #[cfg(feature = "trace")]
        let mut inf_fails = 0;
        while inf_steps < self.inflationary_stage_steps {
            let success = run_step::<N_OUT_INF, N_IN_INF, _, _>(
                self.circuit.num_wires,
                &mut self.circuit.gates,
                LocalMixingStage::Inflationary,
                inf_steps,
                &self.ct,
                self.max_circuit_samples,
                &mut tracer,
                rng,
            );
            if success {
                inf_steps += 1;
            } else {
                #[cfg(feature = "trace")]
                {
                    inf_fails += 1;
                }
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
        let mut knd_steps = 1;
        #[cfg(feature = "trace")]
        let mut knd_fails = 0;
        while knd_steps <= self.kneading_stage_steps {
            let success = run_step::<N_OUT_KND, N_IN_KND, _, _>(
                self.circuit.num_wires,
                &mut self.circuit.gates[..],
                LocalMixingStage::Kneading,
                knd_steps,
                &self.ct,
                self.max_circuit_samples,
                &mut tracer,
                rng,
            );
            if success {
                knd_steps += 1;
                if knd_steps % EPOCH_SIZE == 0 {
                    self.circuit
                    .save_as_json(format!("{}/save-{}-kneading.json", self.dir_path, knd_steps));

                #[cfg(feature = "trace")]
                {
                    self.circuit
                        .save_generation_data(format!("{}/generation.json", self.dir_path));

                    tracer
                        .save_to_file(format!("{}/logs/trace.json", self.dir_path))
                        .expect("Failed to save trace");
                    log::info!(target: "trace", "Saved at step {}", knd_steps);
                }
                }
            } else {
                #[cfg(feature = "trace")]
                {
                    knd_fails += 1;
                }
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
                .save_to_file(format!("{}/logs/trace.json", self.dir_path))
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
        #[cfg(feature = "trace")]
        let mut inf_fails = 0;
        while inf_steps < self.inflationary_stage_steps {
            let success = run_step::<N_OUT_INF, N_IN_INF, _, _>(
                self.circuit.num_wires,
                &mut self.circuit.gates,
                LocalMixingStage::Inflationary,
                inf_steps,
                &self.ct,
                self.max_circuit_samples,
                &mut inf_tracer,
                rng,
            );
            if success {
                inf_steps += 1;
            } else {
                #[cfg(feature = "trace")]
                {
                    inf_fails += 1;
                }
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

        let num_search_workers = min(self.search_threads, current_num_threads());
        println!("-- Using {} search threads", num_search_workers);

        let num_gates = self.circuit.gates.len();
        let chunk_size = num_gates / num_search_workers;
        let phase_bds = {
            let middle_size = chunk_size * (num_search_workers - 2);
            let first_last_size = num_gates - middle_size;
            let chunk_equal = first_last_size / 2;
            let chunk_equal_other = first_last_size - chunk_equal;
            let chunk_smaller = first_last_size / 4;
            let chunk_bigger = first_last_size - chunk_smaller;
            [
                [chunk_equal, chunk_equal_other],
                [chunk_smaller, chunk_bigger],
                [chunk_bigger, chunk_smaller],
            ]
        };

        let mut knd_tracers = Vec::with_capacity(num_search_workers);
        for _ in 0..num_search_workers {
            knd_tracers.push(Tracer::new(0, self.kneading_stage_steps));
        }
        let mut rngs: Vec<_> = (0..num_search_workers)
            .map(|_| ChaCha8Rng::from_os_rng())
            .collect();

        let mut knd_steps = 0;
        #[cfg(feature = "trace")]
        let mut knd_fails = 0;
        let mut epoch_steps = 0;

        while knd_steps < self.kneading_stage_steps {
            for first_last_bds in phase_bds {
                let mut chunks = vec![];
                let (rest, last) = self
                    .circuit
                    .gates
                    .split_at_mut(num_gates - first_last_bds[1]);
                let (first, middle) = rest.split_at_mut(first_last_bds[0]);
                chunks.push(first);
                middle
                    .chunks_mut(chunk_size)
                    .for_each(|chunk| chunks.push(chunk));
                chunks.push(last);

                let num_success = chunks
                    .par_iter_mut()
                    .zip(knd_tracers.par_iter_mut())
                    .zip(rngs.par_iter_mut())
                    .map(|((chunk, tracer), rng)| {
                        let success = run_step::<N_OUT_KND, N_IN_KND, _, _>(
                            self.circuit.num_wires,
                            chunk,
                            LocalMixingStage::Kneading,
                            knd_steps,
                            &self.ct,
                            self.max_circuit_samples,
                            tracer,
                            rng,
                        );
                        success
                    })
                    .filter(|&success| success)
                    .count();

                knd_steps += num_success;
                #[cfg(feature = "trace")]
                {
                    knd_fails += num_search_workers - num_success;
                }
                epoch_steps += num_success;
            }

            if epoch_steps > EPOCH_SIZE {
                epoch_steps = 0;

                self.circuit
                    .save_as_json(format!("{}/save-{}-kneading.json", self.dir_path, knd_steps));

                #[cfg(feature = "trace")]
                {
                    self.circuit
                        .save_generation_data(format!("{}/generation.json", self.dir_path));

                    let mut all_tracers = vec![inf_tracer.clone()];
                    all_tracers.extend(knd_tracers.clone());
                    let tracer = Tracer::collect(all_tracers.into_iter());
                    tracer
                        .save_to_file(format!("{}/logs/trace.json", self.dir_path))
                        .expect("Failed to save trace");
                    log::info!(target: "trace", "Saved at step {}", knd_steps);
                }
            }
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
            let tracer = Tracer::collect(all_tracers.into_iter());
            tracer
                .save_to_file(format!("{}/logs/trace.json", self.dir_path))
                .expect("Failed to save trace");
            log::info!(target: "trace", "Finished.");
            log::info!(target: "trace", "Inflationary stage fails: {}", inf_fails);
            log::info!(target: "trace", "Kneading stage fails: {}", knd_fails);
        }
    }

    pub fn results(self) -> (Circuit, CompressionTable) {
        (self.circuit, self.ct)
    }

    pub fn experiment_config(dir_path: &str, circuit: Circuit, ct: CompressionTable) -> Self {
        Self { 
            inflationary_stage_steps: 0, 
            kneading_stage_steps: 500000,
            gate_library: GateLibrary::TwoBit, 
            search_threads: 1,
            max_circuit_samples: 10,
            save_inflationary: false,
            compression_table_path: "bin/table-twobit.db".to_string(), 
            num_wires: 16, 
            original_num_gates: 1, 
            circuit, 
            ct, 
            dir_path: dir_path.to_string() 
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

fn run_step<const N_OUT: usize, const N_IN: usize, G: Growable + ?Sized, R: Send + Sync + RngCore + SeedableRng>(
    circuit_num_wires: usize,
    circuit_gates: &mut G,
    stage: LocalMixingStage,
    _current_step: usize,
    ct: &CompressionTable,
    max_circuit_samples: usize,
    _tracer: &mut Tracer,
    rng: &mut R,
) -> bool {
    #[cfg(feature = "trace")]
    let start_time = Instant::now();

    #[cfg(feature = "search-1")]
    let (selected_gate_idx, _n_search_attempts) =
        find_convex_gate_ids3(N_OUT, 9, circuit_num_wires, circuit_gates.as_slice_ref(), rng);

    #[cfg(feature = "search-2")]
    let (selected_gate_idx, _n_search_attempts) =
        find_convex_gate_ids_max_spread(N_OUT, 9, circuit_num_wires, circuit_gates.as_slice_ref(), rng);

    #[cfg(feature = "search-3")]
    let (selected_gate_idx, _n_search_attempts) =
        find_convex_gate_ids_max_spread_overall(10, N_OUT, 9, circuit_num_wires, circuit_gates.as_slice_ref(), rng);

    let c_out = selected_gate_idx
        .iter()
        .map(|i| circuit_gates.gate_at_index(*i))
        .collect::<Vec<_>>();

    #[cfg(feature = "trace")]
    let repl_start = Instant::now();
    
    let replacement_res = match stage {
        LocalMixingStage::Inflationary => find_replacement_random_sample(&c_out, circuit_num_wires, N_IN, 1_000_000_000, ct.gate_library, true, rng),
        LocalMixingStage::Kneading => find_replacement_with_ct(&c_out, circuit_num_wires, N_IN, max_circuit_samples, &ct, false, rng),
    };

    #[cfg(feature = "trace")]
    let _replacement_time = Instant::now() - repl_start;

    if let Some((c_in, _n_circuits_sampled)) = replacement_res {
        #[cfg(feature = "trace")]
        let c_in_trace = c_in.clone();

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

        #[cfg(feature = "correctness")]
        {
            if check_ckt_equiv_inout_map(&inout, circuit_gates.as_slice_ref()) == false {
                let error_str = format!("{}, step={}, Obfuscated circuit is functionally not equivalent to original input circuit", stage,  _current_step);
                log::error!(target: "trace", "{error_str}");
                panic!("{error_str}");
            }
        }

        #[cfg(feature = "trace")]
        {
            let _elapsed = Instant::now() - start_time;
            let n_gates = circuit_gates.as_slice_ref().len();
            let target_idx = c_out_start..c_out_start + N_OUT;

            log::info!(target: "trace", "{}", format!("{}, step={}, SUCCESS: n_gates={}, gate_ids={:?}, target_gate_ids={:?}, n_circuits_sampled={}, n_search_attempts={}, replacement_time={:?}, total_time={:?}", 
                stage, 
                _current_step, 
                n_gates,
                selected_gate_idx,
                target_idx,
                _n_circuits_sampled,
                _n_search_attempts, 
                _replacement_time, 
                _elapsed));

            let search_fields = SearchTraceFields {
                gate_indices: selected_gate_idx.to_vec(),
                n_search_attempts: _n_search_attempts,
            };
            let c_out_data = c_out.iter().map(|&g| GateData::from(g)).collect();
            let c_in_data = c_in_trace.iter().map(|&g| GateData::from(g)).collect();
            let replacement_fields = ReplacementTraceFields {
                data: ReplacementStatus::Success(c_out_data, c_in_data),
                replacement_time: _replacement_time,
                n_circuits_sampled: _n_circuits_sampled,
                min_generation: circuit_min_generation(&c_out),
            };
            _tracer.add_entry(
                stage,
                _current_step,
                search_fields,
                replacement_fields,
                _elapsed,
            );
        }

        return true;
    } else {
        #[cfg(feature = "trace")]
        {
            let _elapsed = Instant::now() - start_time;

            log::warn!(target: "trace", "{}, step={} FAIL, gate_ids={:?}, replacement_time={:?}", stage, _current_step, selected_gate_idx, _replacement_time);

            let search_fields = SearchTraceFields {
                gate_indices: selected_gate_idx.to_vec(),
                n_search_attempts: _n_search_attempts,
            };
            let c_out_data = c_out.iter().map(|&g| GateData::from(g)).collect();
            let replacement_fields = ReplacementTraceFields {
                data: ReplacementStatus::Fail(c_out_data),
                replacement_time: _replacement_time,
                n_circuits_sampled: max_circuit_samples,
                min_generation: circuit_min_generation(&c_out),
            };
            _tracer.add_entry(
                stage,
                _current_step,
                search_fields,
                replacement_fields,
                _elapsed,
            );
        }

        return false;
    }
}
