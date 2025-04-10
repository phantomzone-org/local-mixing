use std::{cmp::min, error::Error, fs::File, io::BufReader, path::Path, time::Instant};

use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use rayon::{
    current_num_threads,
    iter::{IndexedParallelIterator, IntoParallelRefMutIterator, ParallelIterator},
};
use serde::{Deserialize, Serialize};

use crate::{
    circuit::Circuit, compression::ct::CompressionTable, local_mixing::tracer::Tracer,
    replacement::strategy::ControlFnChoice,
};

use super::{
    consts::{DEFAULT_NUM_GATES, DEFAULT_NUM_WIRES},
    tracer::init_logs,
    worker::Worker,
};

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
        let ct_path = "bin/table-twobit.db";

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
                DEFAULT_NUM_WIRES,
                DEFAULT_NUM_GATES,
                &job.cf_choice,
                &mut rng,
            );
            job.circuit.save_as_json(input_circuit_path.clone());
            println!("-- Saved random circuit into {}", input_circuit_path);
        }

        // Load compression table
        println!("-- Loading compression table at {}", ct_path);
        job.ct = CompressionTable::from_file(ct_path);
        assert!(job.cf_choice.cfs() == job.ct.cf_choice);
        println!("-- Loading compression table done");

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

    pub fn run_one_thread(&mut self) {
        let mut worker = Worker::new(
            0,
            self.gate_sample_limit,
            self.inflationary_stage_steps,
            self.kneading_stage_steps,
        );

        println!("-- Inflationary stage");
        worker.run_task(
            &LocalMixingStage::Inflationary,
            self.inflationary_stage_steps,
            self.circuit.clone(),
            &self.ct,
        );
        println!("-- Inflationary stage: done");

        let mut inflationary_circuit = worker.get_current_circuit();
        if self.save_inflationary && self.inflationary_stage_steps > 0 {
            let inflationary_path = format!("{}/inflationary.json", self.dir_path);
            inflationary_circuit.save_as_json(inflationary_path.clone());
            println!("-- Saved inflationary stage to {}", inflationary_path);
        }
        inflationary_circuit.reset_generations();

        println!("-- Kneading stage");
        worker.run_task(
            &LocalMixingStage::Kneading,
            self.kneading_stage_steps,
            inflationary_circuit,
            &self.ct,
        );
        println!("-- Kneading stage: done");

        worker
            .get_current_circuit()
            .save_as_json(format!("{}/target.json", self.dir_path));

        #[cfg(feature = "trace")]
        {
            worker
                .get_current_circuit()
                .save_generation_data(format!("{}/generation.json", self.dir_path));
            worker.save_trace(&self.dir_path);
        }
    }

    pub fn run_multiple_threads(&mut self) {
        let mut inf_worker = Worker::new(
            0,
            self.gate_sample_limit,
            self.inflationary_stage_steps,
            self.kneading_stage_steps,
        );

        println!("-- Inflationary stage");
        inf_worker.run_task(
            &LocalMixingStage::Inflationary,
            self.inflationary_stage_steps,
            self.circuit.clone(),
            &self.ct,
        );
        println!("-- Inflationary stage: done");

        self.circuit = inf_worker.get_current_circuit();
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
        let mut steps_completed = 0;
        while steps_completed < self.kneading_stage_steps {
            let phase1_circuits = self.circuit.split_into_chunks(num_search_workers, 0);
            self.circuit.gates = workers
                .par_iter_mut()
                .enumerate()
                .map(|(i, worker)| {
                    let ckt = phase1_circuits[i].clone();
                    worker.run_task(&LocalMixingStage::Kneading, 1, ckt, &self.ct);

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
                    worker.run_task(&LocalMixingStage::Kneading, 1, ckt, &self.ct);

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

            let tracer = Tracer::collect(workers.iter().map(|worker| worker.tracer()));
            tracer.save_to_file(&self.dir_path).unwrap();
        }
    }
}
