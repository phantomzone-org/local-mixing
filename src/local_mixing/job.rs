use std::{cmp::min, error::Error, fs::File, io::BufReader, path::Path, time::Instant};

use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use rayon::{
    current_num_threads,
    iter::{
        IndexedParallelIterator, IntoParallelRefIterator, IntoParallelRefMutIterator,
        ParallelIterator,
    },
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
    run_parallel: bool,
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
        if self.run_parallel {
            self.run_multiple_threads();
        } else {
            self.run_one_thread();
        }
        let elapsed = Instant::now() - start;
        println!("-- Finished running in {:?}", elapsed);
    }

    pub fn run_one_thread(&mut self) {
        let ct_clone = self.ct.clone();
        let mut worker = Worker::new(
            0,
            ct_clone,
            self.inflationary_stage_steps,
            self.kneading_stage_steps,
        );

        println!("-- Inflationary stage");
        worker.run_task(
            &LocalMixingStage::Inflationary,
            self.inflationary_stage_steps,
            self.circuit.clone(),
        );
        println!("-- Inflationary stage: done");

        let inflationary_circuit = worker.get_current_circuit();
        if self.save_inflationary && self.inflationary_stage_steps > 0 {
            let inflationary_path = format!("{}/inflationary.json", self.dir_path);
            inflationary_circuit.save_as_json(inflationary_path.clone());
            println!("-- Saved inflationary stage to {}", inflationary_path);
        }

        println!("-- Kneading stage");
        worker.run_task(
            &LocalMixingStage::Kneading,
            self.kneading_stage_steps,
            inflationary_circuit,
        );
        println!("-- Kneading stage: done");

        worker
            .get_current_circuit()
            .save_as_json(format!("{}/target.json", self.dir_path));

        #[cfg(feature = "trace")]
        worker.save_trace(&self.dir_path);
    }

    pub fn run_multiple_threads(&mut self) {
        let ct_clone = self.ct.clone();
        let mut inf_worker = Worker::new(
            0,
            ct_clone,
            self.inflationary_stage_steps,
            self.kneading_stage_steps,
        );

        println!("-- Inflationary stage");
        inf_worker.run_task(
            &LocalMixingStage::Inflationary,
            self.inflationary_stage_steps,
            self.circuit.clone(),
        );
        println!("-- Inflationary stage: done");

        let inflationary_circuit = inf_worker.get_current_circuit();
        if self.save_inflationary && self.inflationary_stage_steps > 0 {
            let inflationary_path = format!("{}/inflationary.json", self.dir_path);
            inflationary_circuit.save_as_json(inflationary_path.clone());
            println!("-- Saved inflationary stage to {}", inflationary_path);
        }

        println!("-- Kneading stage");

        let num_search_workers = current_num_threads();
        println!(
            "-- Initializing search workers. {} threads available",
            num_search_workers
        );

        let mut workers: Vec<Worker<ChaCha8Rng>> = (0..num_search_workers)
            .map(|worker_id| {
                let ct_clone = self.ct.clone();
                Worker::new(worker_id, ct_clone, 0, self.kneading_stage_steps)
            })
            .collect();

        let chunk_size = self.circuit.gates.len() / num_search_workers;
        let mut steps_completed = 0;
        while steps_completed < self.kneading_stage_steps {
            let phase1_total_steps = min(10000, self.kneading_stage_steps - steps_completed);
            let phase1_base_steps = phase1_total_steps / num_search_workers;
            let mut phase1_extra = phase1_total_steps % num_search_workers;
            let phase1_steps: Vec<usize> = (0..num_search_workers)
                .map(|_| {
                    if phase1_extra != 0 {
                        let steps = phase1_base_steps + 1;
                        phase1_extra -= 1;
                        return steps;
                    }
                    phase1_base_steps
                })
                .collect();

            let phase1_circuits = self.circuit.split_into_chunks(num_search_workers, 0);

            workers.par_iter_mut().enumerate().for_each(|(i, worker)| {
                let steps = phase1_steps[i];
                let ckt = phase1_circuits[i].clone();
                worker.run_task(&LocalMixingStage::Kneading, steps, ckt);
            });

            self.circuit.gates = workers
                .par_iter()
                .map(|w| w.get_current_circuit())
                .flat_map(|ckt| ckt.gates)
                .collect();
            steps_completed += phase1_total_steps;

            let phase2_total_steps = min(10000, self.kneading_stage_steps - steps_completed);
            let phase2_base_steps = phase2_total_steps / num_search_workers;
            let mut phase2_extra = phase2_total_steps % num_search_workers;
            let phase2_steps: Vec<usize> = (0..num_search_workers)
                .map(|_| {
                    if phase2_extra != 0 {
                        let steps = phase2_base_steps + 1;
                        phase2_extra -= 1;
                        return steps;
                    }
                    phase2_base_steps
                })
                .collect();

            let phase2_circuits = self
                .circuit
                .split_into_chunks(num_search_workers, chunk_size / 2);

            workers.par_iter_mut().enumerate().for_each(|(i, worker)| {
                let steps = phase2_steps[i];
                let ckt = phase2_circuits[i].clone();
                worker.run_task(&LocalMixingStage::Kneading, steps, ckt);
            });

            self.circuit.gates = workers
                .par_iter()
                .map(|w| w.get_current_circuit())
                .flat_map(|ckt| ckt.gates)
                .collect();
            steps_completed += phase2_total_steps;
        }
        println!("-- Kneading stage: done");

        self.circuit
            .save_as_json(format!("{}/target.json", self.dir_path));

        #[cfg(feature = "trace")]
        {
            let tracer = Tracer::collect(workers.iter().map(|worker| worker.tracer()));
            tracer.save_to_file(&self.dir_path).unwrap();
        }
    }
}
