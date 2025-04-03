use serde::{Deserialize, Serialize};
use std::{error::Error, fs::File, time::Duration};

use crate::circuit::Gate;

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct ReplacementTraceFields {
    pub input_circuit: Vec<Gate>,
    pub output_circuit: Vec<Gate>,
    pub num_input_wires: usize,
    pub num_output_wires: usize,
    pub num_active_wires: usize,
    pub min_generation: usize,
    pub num_circuits_sampled: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct SearchTraceFields {
    n_gates: usize,
    max_candidate_dist: usize,
    time: Duration,
    replacement_fields: ReplacementTraceFields,
}

pub enum Stage {
    Inflationary,
    Kneading,
}

impl std::fmt::Display for Stage {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let s = match self {
            Stage::Inflationary => "Inflationary",
            Stage::Kneading => "Kneading",
        };
        write!(f, "{}", s)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct ReplacementTimes {
    inflationary_stage: Vec<Duration>,
    kneading_stage: Vec<Duration>,
}

impl ReplacementTimes {
    fn new(inf_steps: usize, kneading_steps: usize) -> Self {
        Self {
            inflationary_stage: Vec::with_capacity(inf_steps),
            kneading_stage: Vec::with_capacity(kneading_steps),
        }
    }

    fn add_entry(&mut self, stage: &Stage, duration: Duration) {
        match stage {
            Stage::Inflationary => self.inflationary_stage.push(duration),
            Stage::Kneading => self.kneading_stage.push(duration),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct ReplacementInfo {
    inflationary_stage: Vec<ReplacementTraceFields>,
    kneading_stage: Vec<ReplacementTraceFields>,
}

impl ReplacementInfo {
    fn new(inf_steps: usize, kneading_steps: usize) -> Self {
        Self {
            inflationary_stage: Vec::with_capacity(inf_steps),
            kneading_stage: Vec::with_capacity(kneading_steps),
        }
    }

    fn add_entry(&mut self, stage: &Stage, replacement_fields: ReplacementTraceFields) {
        match stage {
            Stage::Inflationary => self.inflationary_stage.push(replacement_fields),
            Stage::Kneading => self.kneading_stage.push(replacement_fields),
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct TracerStash {
    search: Option<SearchTraceFields>,
    replacement: Option<Duration>,
}

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct Tracer {
    dir_path: String,
    pub replacement_times: ReplacementTimes,
    pub replacement_info: ReplacementInfo,
    pub stash: TracerStash,
}

impl Tracer {
    pub fn new(
        dir_path: &String,
        inf_steps: usize,
        kneading_steps: usize,
    ) -> Result<Self, Box<dyn Error>> {
        init_logs(dir_path)?;

        Ok(Self {
            dir_path: dir_path.clone(),
            replacement_times: ReplacementTimes::new(inf_steps, kneading_steps),
            replacement_info: ReplacementInfo::new(inf_steps, kneading_steps),
            stash: TracerStash::default(),
        })
    }

    pub fn add_search_entry(
        &mut self,
        n_gates: usize,
        max_candidate_dist: usize,
        time: Duration,
        replacement_fields: ReplacementTraceFields,
    ) {
        assert!(self.stash.search.is_none());
        self.stash.search = Some(SearchTraceFields {
            n_gates,
            max_candidate_dist,
            time,
            replacement_fields,
        });
    }

    pub fn add_replacement_time(&mut self, time: Duration) {
        assert!(self.stash.replacement.is_none());
        self.stash.replacement = Some(time);
    }

    pub fn flush_stash(&mut self, stage: Stage, step: usize) {
        if let Some(search) = &self.stash.search {
            log::info!(target: "trace", "{}", format!("{} step={}, SUCCESS: n_gates = {}, n_circuits_sampled = {}, max_candidate_dist = {}, time = {:?}, c_in = {:?}, c_out = {:?}", 
            stage, step, search.n_gates, search.replacement_fields.num_circuits_sampled, search.max_candidate_dist, search.time, search.replacement_fields.input_circuit, search.replacement_fields.output_circuit));

            self.replacement_info
                .add_entry(&stage, search.replacement_fields.clone());
        }

        if let Some(duration) = self.stash.replacement {
            self.replacement_times.add_entry(&stage, duration);
        }

        self.stash = TracerStash::default();
    }

    pub fn empty_stash(&mut self) {
        self.stash = TracerStash::default();
    }

    pub fn save_replacement_data(&self) -> Result<(), Box<dyn std::error::Error>> {
        let file = File::create(format!("{}/logs/replacement_times.json", self.dir_path)).unwrap();
        serde_json::to_writer_pretty(file, &self.replacement_times)?;

        let file = File::create(format!("{}/logs/replacement_fields.json", self.dir_path)).unwrap();
        serde_json::to_writer_pretty(file, &self.replacement_info)?;
        Ok(())
    }
}

fn init_logs(dir_path: &String) -> Result<(), Box<dyn std::error::Error>> {
    let trace_file_appender = log4rs::append::file::FileAppender::builder()
        .encoder(Box::new(log4rs::encode::pattern::PatternEncoder::new(
            "{d} - {l} - {m}{n}",
        )))
        .build(&format!("{}/logs/trace.log", dir_path))?;

    let mut config_builder = log4rs::Config::builder();

    config_builder = config_builder.appender(
        log4rs::config::Appender::builder().build("trace", Box::new(trace_file_appender)),
    );
    config_builder = config_builder.logger(
        log4rs::config::Logger::builder()
            .appender("trace")
            .additive(false)
            .build("trace", log::LevelFilter::Trace),
    );

    let mut root_builder = log4rs::config::Root::builder();

    root_builder = root_builder.appender("trace");

    let config = config_builder.build(root_builder.build(log::LevelFilter::Trace))?;

    log4rs::init_config(config)?;

    Ok(())
}
