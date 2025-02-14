use crate::{
    circuit::Circuit,
    local_mixing::{
        consts::{self, N_OUT_INF, N_OUT_KND},
        tracer::Stage,
    },
    replacement::strategy::{ControlFnChoice, ReplacementStrategy},
};
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};
use std::{error::Error, fs::File, io::BufReader};

#[cfg(feature = "correctness")]
use crate::circuit::circuit::is_func_equiv;

#[cfg(feature = "trace")]
use super::tracer::Tracer;

#[derive(Clone, Serialize, Deserialize, Debug)]
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
    pub max_attempts_without_success: usize,
    /// Whether to save to file, logs
    pub save: bool,
    #[serde(default)]
    pub replacement_strategy: ReplacementStrategy,
    /// Control function choice in replacement
    #[serde(default)]
    pub cf_choice: ControlFnChoice,
    /// Whether job is in-progress on loading, determines source for circuit
    #[serde(default)]
    in_progress: bool,
    /// How often circuit is saved to file
    #[serde(default)]
    pub epoch_size: usize,
    /// Current inflationary step
    #[serde(default)]
    pub curr_inflationary_step: usize,
    /// Current kneading step
    #[serde(default)]
    pub curr_kneading_step: usize,
    /// Current circuit
    #[serde(default, skip_serializing)]
    pub circuit: Circuit,
    /// Original input circuit
    #[cfg(feature = "correctness")]
    #[serde(default, skip_serializing)]
    original_circuit: Circuit,
    /// Tracer
    #[cfg(feature = "trace")]
    #[serde(default, skip_serializing)]
    pub tracer: Tracer,
}

impl LocalMixingJob {
    pub fn new(
        wires: u32,
        inflationary_stage_steps: usize,
        kneading_stage_steps: usize,
        max_replacement_samples: usize,
        max_attempts_without_success: usize,
        replacement_strategy: ReplacementStrategy,
        cf_choice: ControlFnChoice,
        circuit: Circuit,
    ) -> Self {
        Self {
            wires,
            inflationary_stage_steps,
            kneading_stage_steps,
            max_replacement_samples,
            max_attempts_without_success,
            replacement_strategy,
            cf_choice,
            circuit: circuit.clone(),
            save: false,
            epoch_size: 0,
            in_progress: false,
            curr_inflationary_step: 0,
            curr_kneading_step: 0,
            #[cfg(feature = "correctness")]
            original_circuit: circuit,
            #[cfg(feature = "trace")]
            tracer: Tracer::default(),
        }
    }

    pub fn load(dir_path: &String) -> Result<Self, Box<dyn Error>> {
        let config_path = format!("{}/config.json", dir_path);
        let file = File::open(&config_path)?;
        let reader = BufReader::new(file);
        let mut job: Self = serde_json::from_reader(reader)?;

        let circuit_file_name = if job.in_progress {
            "save.bin"
        } else {
            "input.bin"
        };
        job.circuit = Circuit::load_from_binary(format!("{}/{}", dir_path, circuit_file_name))?;

        #[cfg(feature = "correctness")]
        {
            job.original_circuit = Circuit::load_from_binary(format!("{}/input.bin", dir_path))?;
        }

        #[cfg(feature = "trace")]
        {
            job.tracer = Tracer::new(
                dir_path,
                job.inflationary_stage_steps,
                job.kneading_stage_steps,
            )?;
        }

        Ok(job)
    }

    pub fn save(&self, dir_path: &String) {
        self.circuit
            .save_as_binary(format!("{}/save.bin", dir_path));
        let file = File::create(format!("{}/config.json", dir_path)).unwrap();
        serde_json::to_writer_pretty(file, &self).unwrap();
    }

    pub fn execute(&mut self, dir_path: &String) -> bool {
        let mut iter = 1;
        let mut fail_ctr = 0;
        let mut rng = ChaCha8Rng::from_os_rng();

        self.in_progress = true;

        while self.in_inflationary_stage() {
            let success = self.execute_step::<_, N_OUT_INF>(&mut rng);
            match success {
                Ok(()) => {
                    #[cfg(feature = "correctness")]
                    assert!(
                        is_func_equiv(
                            &self.original_circuit,
                            &self.circuit,
                            consts::CORRECTNESS_CHECK_ITER,
                            &mut rng
                        ) == Ok(())
                    );

                    #[cfg(any(feature = "trace"))]
                    self.tracer
                        .flush_stash(Stage::Inflationary, self.curr_inflationary_step);

                    self.curr_inflationary_step += 1;

                    // Save snapshot every epoch
                    if self.save && iter % self.epoch_size == 0 {
                        self.save(dir_path);
                    }

                    iter += 1;
                    fail_ctr = 0;
                }
                Err(_e) => {
                    #[cfg(feature = "trace")]
                    {
                        log::warn!(target: "trace", "{}, FAILED: {}", Stage::Inflationary, _e);
                        // empty the stash if the step failed
                        self.tracer.empty_stash();
                    }

                    fail_ctr += 1;
                    if fail_ctr == self.max_attempts_without_success {
                        return false;
                    }
                }
            }
        }

        #[cfg(feature = "trace")]
        let _ = self.tracer.save_replacement_time().inspect_err(
            |e| log::warn!(target: "trace", "{}, Failed to store replacement times with error: {}", Stage::Inflationary, e),
        );

        while self.in_kneading_stage() {
            let success = self.execute_step::<_, N_OUT_KND>(&mut rng);
            match success {
                Ok(()) => {
                    #[cfg(feature = "correctness")]
                    assert!(
                        is_func_equiv(
                            &self.original_circuit,
                            &self.circuit,
                            consts::CORRECTNESS_CHECK_ITER,
                            &mut rng
                        ) == Ok(())
                    );

                    #[cfg(any(feature = "trace"))]
                    self.tracer
                        .flush_stash(Stage::Kneading, self.curr_kneading_step);

                    self.curr_kneading_step += 1;

                    if self.save && iter % self.epoch_size == 0 {
                        self.save(&dir_path);
                    }

                    iter += 1;
                    fail_ctr = 0;
                }
                Err(_e) => {
                    #[cfg(feature = "trace")]
                    {
                        log::warn!(target: "trace", "{}, FAILED: {}", Stage::Kneading, _e);
                        // empty the stash if the step failed
                        self.tracer.empty_stash();
                    }

                    fail_ctr += 1;
                    if fail_ctr == self.max_attempts_without_success {
                        return false;
                    }
                }
            }
        }

        // Local mixing successful
        self.circuit
            .save_as_binary(format!("{}/target.bin", dir_path));
        if self.save {
            self.save(dir_path);
        }

        #[cfg(feature = "trace")]
        let _ = self.tracer.save_replacement_time().inspect_err(
            |e| log::warn!(target: "trace", "{}, Failed to store replacement times with error: {}",Stage::Kneading, e),
        );

        return true;
    }

    fn in_inflationary_stage(&self) -> bool {
        self.curr_inflationary_step < self.inflationary_stage_steps
    }

    fn in_kneading_stage(&self) -> bool {
        self.curr_kneading_step < self.kneading_stage_steps
    }
}
