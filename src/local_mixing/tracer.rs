use std::{error::Error, fs::File, time::Duration};

use serde::{Deserialize, Serialize};

use crate::circuit::{circuit::GateData, Gate};

use super::job::LocalMixingStage;

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct ReplacementTraceFields {
    pub num_input_wires: usize,
    pub num_output_wires: usize,
    pub num_active_wires: usize,
    pub min_generation: usize,
    pub num_circuits_sampled: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct SearchTraceFields {
    pub n_gates: usize,
    pub n_search_attempts: usize,
    pub time: Duration,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct ReplacementTimes {
    inflationary_stage: Vec<Duration>,
    kneading_stage: Vec<Duration>,
}

impl ReplacementTimes {
    fn new(inf_capacity: usize, knd_capacity: usize) -> Self {
        Self {
            inflationary_stage: Vec::with_capacity(inf_capacity),
            kneading_stage: Vec::with_capacity(knd_capacity),
        }
    }

    fn add_entry(&mut self, stage: LocalMixingStage, duration: Duration) {
        match stage {
            LocalMixingStage::Inflationary => self.inflationary_stage.push(duration),
            LocalMixingStage::Kneading => self.kneading_stage.push(duration),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct ReplacementInfo {
    inflationary_stage: Vec<ReplacementTraceFields>,
    kneading_stage: Vec<ReplacementTraceFields>,
}

impl ReplacementInfo {
    fn new(inf_capacity: usize, knd_capacity: usize) -> Self {
        Self {
            inflationary_stage: Vec::with_capacity(inf_capacity),
            kneading_stage: Vec::with_capacity(knd_capacity),
        }
    }

    fn add_entry(&mut self, stage: LocalMixingStage, replacement_fields: ReplacementTraceFields) {
        match stage {
            LocalMixingStage::Inflationary => self.inflationary_stage.push(replacement_fields),
            LocalMixingStage::Kneading => self.kneading_stage.push(replacement_fields),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct SearchInfo {
    inflationary_stage: Vec<SearchTraceFields>,
    kneading_stage: Vec<SearchTraceFields>,
}

impl SearchInfo {
    fn new(inf_capacity: usize, knd_capacity: usize) -> Self {
        Self {
            inflationary_stage: Vec::with_capacity(inf_capacity),
            kneading_stage: Vec::with_capacity(knd_capacity),
        }
    }

    fn add_entry(&mut self, stage: LocalMixingStage, search_fields: SearchTraceFields) {
        match stage {
            LocalMixingStage::Inflationary => self.inflationary_stage.push(search_fields),
            LocalMixingStage::Kneading => self.kneading_stage.push(search_fields),
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ReplacementSampleFields {
    pub input: Vec<GateData>,
    pub output: Vec<GateData>,
    current_step: usize,
}

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct ReplacementSamples {
    pub inflationary_stage: Vec<ReplacementSampleFields>,
    pub kneading_stage: Vec<ReplacementSampleFields>,
}

impl ReplacementSamples {
    fn new(inf_capacity: usize, knd_capacity: usize) -> Self {
        Self {
            inflationary_stage: Vec::with_capacity(inf_capacity),
            kneading_stage: Vec::with_capacity(knd_capacity),
        }
    }

    fn add_entry(&mut self, stage: LocalMixingStage, replacement: ReplacementSampleFields) {
        match stage {
            LocalMixingStage::Inflationary => self.inflationary_stage.push(replacement),
            LocalMixingStage::Kneading => self.kneading_stage.push(replacement),
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ReplacementFailFields {
    pub circuit: Vec<GateData>,
    current_step: usize,
}

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct ReplacementFails {
    pub inflationary_stage: Vec<ReplacementFailFields>,
    pub kneading_stage: Vec<ReplacementFailFields>,
}

impl ReplacementFails {
    fn new(inf_capacity: usize, knd_capacity: usize) -> Self {
        Self {
            inflationary_stage: Vec::with_capacity(inf_capacity),
            kneading_stage: Vec::with_capacity(knd_capacity),
        }
    }

    fn add_entry(&mut self, stage: LocalMixingStage, input: Vec<Gate>, current_step: usize) {
        let input_data = input.iter().map(|&g| GateData::from(g)).collect();
        match stage {
            LocalMixingStage::Inflationary => self.inflationary_stage.push(ReplacementFailFields {
                circuit: input_data,
                current_step,
            }),
            LocalMixingStage::Kneading => self.kneading_stage.push(ReplacementFailFields {
                circuit: input_data,
                current_step,
            }),
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct Tracer {
    pub replacement_times: ReplacementTimes,
    pub replacement_info: ReplacementInfo,
    pub search_info: SearchInfo,
    pub replacement_samples: ReplacementSamples,
    pub replacement_fails: ReplacementFails,
}

impl Tracer {
    pub fn new(inf_capacity: usize, knd_capacity: usize) -> Self {
        Self {
            replacement_times: ReplacementTimes::new(inf_capacity, knd_capacity),
            replacement_info: ReplacementInfo::new(inf_capacity, knd_capacity),
            search_info: SearchInfo::new(inf_capacity, knd_capacity),
            replacement_samples: ReplacementSamples::new(inf_capacity, knd_capacity),
            replacement_fails: ReplacementFails::new(inf_capacity, knd_capacity),
        }
    }

    pub fn add_entry(
        &mut self,
        stage: LocalMixingStage,
        search_fields: SearchTraceFields,
        replacement_fields: ReplacementTraceFields,
        replacement_time: Duration,
    ) {
        self.replacement_times.add_entry(stage, replacement_time);
        self.replacement_info.add_entry(stage, replacement_fields);
        self.search_info.add_entry(stage, search_fields);
    }

    pub fn add_replacement_sample(
        &mut self,
        stage: LocalMixingStage,
        input: Vec<Gate>,
        output: Vec<Gate>,
        current_step: usize,
    ) {
        self.replacement_samples.add_entry(
            stage,
            ReplacementSampleFields {
                input: input.iter().map(|&g| g.into()).collect(),
                output: output.iter().map(|&g| g.into()).collect(),
                current_step,
            },
        );
    }

    pub fn add_failed_replacement(
        &mut self,
        stage: LocalMixingStage,
        input: Vec<Gate>,
        current_step: usize,
    ) {
        self.replacement_fails.add_entry(stage, input, current_step);
    }

    pub fn save_to_file(&self, dir_path: &str) -> Result<(), Box<dyn Error>> {
        let file = File::create(format!("{}/logs/replacement_times.json", dir_path)).unwrap();
        serde_json::to_writer_pretty(file, &self.replacement_times)?;

        let file = File::create(format!("{}/logs/replacement_fields.json", dir_path)).unwrap();
        serde_json::to_writer_pretty(file, &self.replacement_info)?;

        let file = File::create(format!("{}/logs/replacement_samples.json", dir_path)).unwrap();
        serde_json::to_writer_pretty(file, &self.replacement_samples)?;

        let file = File::create(format!("{}/logs/replacement_fails.json", dir_path)).unwrap();
        serde_json::to_writer_pretty(file, &self.replacement_fails)?;

        Ok(())
    }

    pub fn collect(tracers: Vec<Tracer>) -> Self {
        let mut combined = Tracer::new(0, 0);

        for tracer in tracers {
            combined
                .replacement_times
                .inflationary_stage
                .extend(tracer.replacement_times.inflationary_stage);
            combined
                .replacement_times
                .kneading_stage
                .extend(tracer.replacement_times.kneading_stage);

            combined
                .replacement_info
                .inflationary_stage
                .extend(tracer.replacement_info.inflationary_stage);
            combined
                .replacement_info
                .kneading_stage
                .extend(tracer.replacement_info.kneading_stage);

            combined
                .search_info
                .inflationary_stage
                .extend(tracer.search_info.inflationary_stage);
            combined
                .search_info
                .kneading_stage
                .extend(tracer.search_info.kneading_stage);

            combined
                .replacement_samples
                .inflationary_stage
                .extend(tracer.replacement_samples.inflationary_stage);
            combined
                .replacement_samples
                .kneading_stage
                .extend(tracer.replacement_samples.kneading_stage);

            combined
                .replacement_fails
                .inflationary_stage
                .extend(tracer.replacement_fails.inflationary_stage);
            combined
                .replacement_fails
                .kneading_stage
                .extend(tracer.replacement_fails.kneading_stage);
        }

        combined
    }
}

pub fn init_logs(dir_path: &str) -> Result<(), Box<dyn std::error::Error>> {
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
