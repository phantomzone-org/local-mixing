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

    fn add_entry(&mut self, stage: &LocalMixingStage, duration: Duration) {
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

    fn add_entry(&mut self, stage: &LocalMixingStage, replacement_fields: ReplacementTraceFields) {
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

    fn add_entry(&mut self, stage: &LocalMixingStage, search_fields: SearchTraceFields) {
        match stage {
            LocalMixingStage::Inflationary => self.inflationary_stage.push(search_fields),
            LocalMixingStage::Kneading => self.kneading_stage.push(search_fields),
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ReplacementSampleFields {
    c_out: Vec<GateData>,
    c_in: Vec<GateData>,
}

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct ReplacementSamples {
    inflationary_stage: Vec<ReplacementSampleFields>,
    kneading_stage: Vec<ReplacementSampleFields>,
}

impl ReplacementSamples {
    fn new(inf_capacity: usize, knd_capacity: usize) -> Self {
        Self {
            inflationary_stage: Vec::with_capacity(inf_capacity),
            kneading_stage: Vec::with_capacity(knd_capacity),
        }
    }

    fn add_entry(&mut self, stage: &LocalMixingStage, replacement: ReplacementSampleFields) {
        match stage {
            LocalMixingStage::Inflationary => self.inflationary_stage.push(replacement),
            LocalMixingStage::Kneading => self.kneading_stage.push(replacement),
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct StepStatusCounter {
    pub success: usize,
    pub fail: usize,
}

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct StepStatuses {
    pub inflationary_stage: StepStatusCounter,
    pub kneading_stage: StepStatusCounter,
}

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct Tracer {
    pub replacement_times: ReplacementTimes,
    pub replacement_info: ReplacementInfo,
    pub search_info: SearchInfo,
    pub replacement_samples: ReplacementSamples,
    pub step_statuses: StepStatuses,
}

impl Tracer {
    pub fn new(inf_capacity: usize, knd_capacity: usize) -> Self {
        Self {
            replacement_times: ReplacementTimes::new(inf_capacity, knd_capacity),
            replacement_info: ReplacementInfo::new(inf_capacity, knd_capacity),
            search_info: SearchInfo::new(inf_capacity, knd_capacity),
            replacement_samples: ReplacementSamples::new(inf_capacity, knd_capacity),
            step_statuses: StepStatuses::default(),
        }
    }

    pub fn add_entry(
        &mut self,
        stage: &LocalMixingStage,
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
        stage: &LocalMixingStage,
        c_out: Vec<Gate>,
        c_in: Vec<Gate>,
    ) {
        self.replacement_samples.add_entry(
            stage,
            ReplacementSampleFields {
                c_out: c_out.iter().map(|&g| g.into()).collect(),
                c_in: c_in.iter().map(|&g| g.into()).collect(),
            },
        );
    }

    pub fn inc_success(&mut self, stage: &LocalMixingStage) {
        match stage {
            LocalMixingStage::Inflationary => self.step_statuses.inflationary_stage.success += 1,
            LocalMixingStage::Kneading => self.step_statuses.kneading_stage.success += 1,
        }
    }

    pub fn inc_fail(&mut self, stage: &LocalMixingStage) {
        match stage {
            LocalMixingStage::Inflationary => self.step_statuses.inflationary_stage.fail += 1,
            LocalMixingStage::Kneading => self.step_statuses.kneading_stage.fail += 1,
        }
    }

    pub fn save_to_file(&self, dir_path: &str) -> Result<(), Box<dyn Error>> {
        let file = File::create(format!("{}/logs/replacement_times.json", dir_path)).unwrap();
        serde_json::to_writer(file, &self.replacement_times)?;

        let file = File::create(format!("{}/logs/replacement_fields.json", dir_path)).unwrap();
        serde_json::to_writer_pretty(file, &self.replacement_info)?;

        let file = File::create(format!("{}/logs/replacement_samples.json", dir_path)).unwrap();
        serde_json::to_writer(file, &self.replacement_samples)?;

        Ok(())
    }

    pub fn collect(tracers: impl Iterator<Item = Tracer>) -> Self {
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

            combined.step_statuses.inflationary_stage.fail +=
                tracer.step_statuses.inflationary_stage.fail;
            combined.step_statuses.inflationary_stage.success +=
                tracer.step_statuses.inflationary_stage.success;
            combined.step_statuses.kneading_stage.fail += tracer.step_statuses.kneading_stage.fail;
            combined.step_statuses.kneading_stage.success +=
                tracer.step_statuses.kneading_stage.success;
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
