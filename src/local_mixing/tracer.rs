use serde::{Deserialize, Serialize};
use std::{error::Error, fs::File, time::Duration};

#[derive(Copy, Clone, Debug, Serialize, Deserialize, Default)]
pub struct SearchTraceFields {
    n_gates: usize,
    n_circuits_sampled: usize,
    max_candidate_dist: usize,
    time: Duration,
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

    fn add_entry(&mut self, stage: Stage, duration: Duration) {
        match stage {
            Stage::Inflationary => self.inflationary_stage.push(duration),
            Stage::Kneading => self.kneading_stage.push(duration),
        }
    }
}

#[derive(Copy, Clone, Serialize, Deserialize, Debug, Default)]
pub struct TracerStash {
    search: Option<SearchTraceFields>,
    replacement: Option<Duration>,
}

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct Tracer {
    dir_path: String,
    pub replacement_times: ReplacementTimes,
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
            stash: TracerStash::default(),
        })
    }

    pub fn add_search_entry(
        &mut self,
        n_gates: usize,
        n_circuits_sampled: usize,
        max_candidate_dist: usize,
        time: Duration,
    ) {
        assert!(self.stash.search.is_none());
        self.stash.search = Some(SearchTraceFields {
            n_gates,
            n_circuits_sampled,
            max_candidate_dist,
            time,
        });
    }

    pub fn add_replacement_time(&mut self, time: Duration) {
        assert!(self.stash.replacement.is_none());
        self.stash.replacement = Some(time);
    }

    pub fn flush_stash(&mut self, stage: Stage) {
        if let Some(search) = self.stash.search {
            log::info!(target: "trace", "{}", format!("{}, SUCCESS: n_gates = {}, n_circuits_sampled = {}, max_candidate_dist = {}, time = {:?}", 
            stage, search.n_gates, search.n_circuits_sampled, search.max_candidate_dist, search.time));
        }

        if let Some(duration) = self.stash.replacement {
            self.replacement_times.add_entry(stage, duration);
        }

        self.stash = TracerStash::default();
    }

    pub fn save_replacement_time(&self) -> Result<(), Box<dyn std::error::Error>> {
        let file = File::create(format!("{}/logs/replacement.json", self.dir_path)).unwrap();
        serde_json::to_writer_pretty(file, &self.replacement_times)?;
        Ok(())
    }
}

pub fn init_logs(dir_path: &String) -> Result<(), Box<dyn std::error::Error>> {
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
