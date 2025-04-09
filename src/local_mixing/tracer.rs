use std::{error::Error, fs::File, time::Duration};

use serde::{Deserialize, Serialize};

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
    pub max_candidate_dist: usize,
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

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct Tracer {
    pub replacement_times: ReplacementTimes,
    pub replacement_info: ReplacementInfo,
    pub search_info: SearchInfo,
}

impl Tracer {
    pub fn new(inf_capacity: usize, knd_capacity: usize) -> Self {
        Self {
            replacement_times: ReplacementTimes::new(inf_capacity, knd_capacity),
            replacement_info: ReplacementInfo::new(inf_capacity, knd_capacity),
            search_info: SearchInfo::new(inf_capacity, knd_capacity),
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

    pub fn save_to_file(&self, dir_path: &str) -> Result<(), Box<dyn Error>> {
        let file = File::create(format!("{}/logs/replacement_times.json", dir_path)).unwrap();
        serde_json::to_writer_pretty(file, &self.replacement_times)?;

        let file = File::create(format!("{}/logs/replacement_fields.json", dir_path)).unwrap();
        serde_json::to_writer_pretty(file, &self.replacement_info)?;
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
