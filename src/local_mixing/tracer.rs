use std::{error::Error, time::Duration};

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct SearchTraceFields {
    n_gates: usize,
    n_circuits_sampled: usize,
    max_candidate_dist: usize,
    time: Duration,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Tracer {
    pub replacement_times: Vec<Duration>,
    pub last_search_logs: SearchTraceFields,
}

impl Tracer {
    pub fn new(dir_path: &String, capacity: usize) -> Result<Self, Box<dyn Error>> {
        init_logs(dir_path)?;

        Ok(Self {
            replacement_times: Vec::with_capacity(capacity),
            last_search_logs: SearchTraceFields::default(),
        })
    }

    pub fn add_search_entry(
        &mut self,
        n_gates: usize,
        n_circuits_sampled: usize,
        max_candidate_dist: usize,
        time: Duration,
    ) {
        self.last_search_logs = SearchTraceFields {
            n_gates,
            n_circuits_sampled,
            max_candidate_dist,
            time,
        };
    }

    pub fn log_last_search_logs(&self, stage: String) {
        log::info!(target: "trace", "{}", format!("{}, SUCCESS: n_gates = {}, n_circuits_sampled = {}, max_candidate_dist = {}, time = {:?}", 
        stage, self.last_search_logs.n_gates, self.last_search_logs.n_circuits_sampled, self.last_search_logs.max_candidate_dist, self.last_search_logs.time));
    }
}

pub fn init_logs(dir_path: &String) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(feature = "trace")]
    let trace_file_appender = log4rs::append::file::FileAppender::builder()
        .encoder(Box::new(log4rs::encode::pattern::PatternEncoder::new(
            "{d} - {l} - {m}{n}",
        )))
        .build(&format!("{}/logs/trace.log", dir_path))?;

    #[cfg(feature = "time")]
    let replacement_file_appender = log4rs::append::file::FileAppender::builder()
        .encoder(Box::new(log4rs::encode::pattern::PatternEncoder::new(
            "{d} - {l} - {m}{n}",
        )))
        .build(&format!("{}/logs/replacement.log", dir_path))?;

    let mut config_builder = log4rs::Config::builder();

    #[cfg(feature = "trace")]
    {
        config_builder = config_builder.appender(
            log4rs::config::Appender::builder().build("trace", Box::new(trace_file_appender)),
        );
        config_builder = config_builder.logger(
            log4rs::config::Logger::builder()
                .appender("trace")
                .additive(false)
                .build("trace", log::LevelFilter::Trace),
        );
    }

    #[cfg(feature = "time")]
    {
        config_builder = config_builder.appender(
            log4rs::config::Appender::builder()
                .build("replacement", Box::new(replacement_file_appender)),
        );
        config_builder = config_builder.logger(
            log4rs::config::Logger::builder()
                .appender("replacement")
                .additive(false)
                .build("replacement", log::LevelFilter::Trace),
        );
    }

    let mut root_builder = log4rs::config::Root::builder();

    #[cfg(feature = "trace")]
    {
        root_builder = root_builder.appender("trace");
    }

    #[cfg(feature = "time")]
    {
        root_builder = root_builder.appender("replacement");
    }

    let config = config_builder.build(root_builder.build(log::LevelFilter::Trace))?;

    log4rs::init_config(config)?;

    Ok(())
}
