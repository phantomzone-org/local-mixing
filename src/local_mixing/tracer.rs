use super::job::LocalMixingStage;
use crate::circuit::circuit::GateData;
use serde::{Deserialize, Serialize};
use std::{error::Error, fs::File, path::Path, time::Duration};

#[derive(Clone, Serialize, Deserialize)]
pub enum ReplacementStatus {
    Success(Vec<GateData>, Vec<GateData>),
    Fail(Vec<GateData>),
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ReplacementTraceFields {
    pub data: ReplacementStatus,
    pub replacement_time: Duration,
    pub n_circuits_sampled: usize,
    pub min_generation: usize,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct SearchTraceFields {
    pub gate_indices: Vec<usize>,
    pub n_search_attempts: usize,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct LocalMixingStepFields {
    pub current_step: usize,
    pub search_fields: SearchTraceFields,
    pub replacement_fields: ReplacementTraceFields,
    pub elapsed: Duration,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Tracer {
    pub inflationary_stage: Vec<LocalMixingStepFields>,
    pub kneading_stage: Vec<LocalMixingStepFields>,
}

impl Tracer {
    pub fn new(inf_capacity: usize, knd_capacity: usize) -> Self {
        Self {
            inflationary_stage: Vec::with_capacity(inf_capacity),
            kneading_stage: Vec::with_capacity(knd_capacity),
        }
    }

    pub fn add_entry(
        &mut self,
        stage: LocalMixingStage,
        current_step: usize,
        search_fields: SearchTraceFields,
        replacement_fields: ReplacementTraceFields,
        elapsed: Duration,
    ) {
        let entry = LocalMixingStepFields {
            current_step,
            search_fields,
            replacement_fields,
            elapsed,
        };
        match stage {
            LocalMixingStage::Inflationary => self.inflationary_stage.push(entry),
            LocalMixingStage::Kneading => self.kneading_stage.push(entry),
        };
    }

    pub fn save_to_file(&self, path: impl AsRef<Path>) -> Result<(), Box<dyn Error>> {
        let file = File::create(path)?;
        serde_json::to_writer(file, &self)?;
        Ok(())
    }

    pub fn collect(tracers: impl Iterator<Item = Tracer>) -> Self {
        let (inf, knd) = tracers.fold(
            (Vec::new(), Vec::new()),
            |(mut inf_acc, mut knd_acc), tracer| {
                inf_acc.extend(tracer.inflationary_stage);
                knd_acc.extend(tracer.kneading_stage);
                (inf_acc, knd_acc)
            },
        );
        Self {
            inflationary_stage: inf,
            kneading_stage: knd,
        }
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
