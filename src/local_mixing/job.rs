use crate::circuit::Circuit;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};
use std::{fs::File, io::BufReader};

#[derive(Serialize, Deserialize, Debug)]
pub struct LocalMixingJob {
    /// Number of wires in circuit
    pub wires: u32,
    /// Number of inflationary steps
    pub inflationary_stage_steps: usize,
    /// Number of kneading steps
    pub kneading_stage_steps: usize,
    /// Max number of attempts to sample func equiv circuit
    pub max_replacement_samples: usize,
    /// Max number of failed replacements before quitting
    max_attempts_without_success: usize,
    /// How often circuit is saved to file
    pub epoch_size: usize,
    /// Path of input circuit
    pub input_circuit_path: String,
    /// Path for storing obfuscated circuit
    pub destination_circuit_path: String,
    /// Path for storing intermediary steps
    pub save_circuit_path: String,
    /// Current inflationary step
    #[serde(default)]
    pub curr_inflationary_step: usize,
    /// Current kneading step
    #[serde(default)]
    pub curr_kneading_step: usize,
    /// Input circuit
    #[serde(default, skip_serializing)]
    pub original_circuit: Circuit,
    /// Current circuit
    #[serde(default, skip_serializing)]
    pub circuit: Circuit,
    /// Path of config sourced
    #[serde(default, skip_serializing)]
    config_path: String,
}

impl LocalMixingJob {
    pub fn load(path: String) -> Self {
        let file = File::open(&path).unwrap();
        let reader = BufReader::new(file);
        let mut job: LocalMixingJob = serde_json::from_reader(reader).unwrap();
        job.original_circuit = Circuit::load_from_binary(&job.input_circuit_path);
        job.circuit = job.original_circuit.clone();
        job.config_path = path;
        job
    }

    pub fn save(&self) {
        self.circuit.save_as_binary(&self.save_circuit_path);
        let file = File::create(self.config_path.clone()).unwrap();
        serde_json::to_writer(file, &self).unwrap();
    }

    pub fn execute(&mut self) -> bool {
        let mut iter = 1;
        let mut fail_ctr = 0;
        let mut rng = ChaCha8Rng::from_entropy();

        while self.in_inflationary_stage() {
            log::info!("Inflationary stage step {}", self.curr_inflationary_step);
            let success = self.execute_step::<_, 2, 4, 9, { 1 << 9 }>(&mut rng);
            if success {
                self.curr_inflationary_step += 1;

                // Save snapshot every epoch
                if iter % self.epoch_size == 0 {
                    self.save();
                }

                iter += 1;
                fail_ctr = 0;
            } else {
                log::warn!("FAILED");
                fail_ctr += 1;

                if fail_ctr == self.max_attempts_without_success {
                    return false;
                }
            }
        }

        while self.in_kneading_stage() {
            log::info!("Kneading stage step {}", self.curr_inflationary_step);
            let success = self.execute_step::<_, 4, 4, 9, { 1 << 9 }>(&mut rng);
            if success {
                self.curr_kneading_step += 1;

                if iter % self.epoch_size == 0 {
                    self.save();
                }

                iter += 1;
                fail_ctr = 0;
            } else {
                log::warn!("FAILED");
                fail_ctr += 1;

                if fail_ctr == self.max_attempts_without_success {
                    return false;
                }
            }
        }

        // Local mixing successful
        self.save();
        self.circuit.save_as_binary(&self.destination_circuit_path);
        return true;
    }

    fn in_inflationary_stage(&self) -> bool {
        self.curr_inflationary_step < self.inflationary_stage_steps
    }

    fn in_kneading_stage(&self) -> bool {
        self.curr_kneading_step < self.kneading_stage_steps
    }
}
