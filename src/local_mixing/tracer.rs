use std::error::Error;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Default)]
pub struct Tracer {}

impl Tracer {
    pub fn new(dir_path: &String) -> Result<Self, Box<dyn Error>> {
        init_logs(dir_path)?;

        Ok(Self {})
    }
}

pub fn init_logs(dir_path: &String) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(feature = "trace")]
    let trace_file_appender = log4rs::append::file::FileAppender::builder()
        .encoder(Box::new(log4rs::encode::pattern::PatternEncoder::new(
            "{d} - {l} - {m}{n}",
        )))
        .build(&format!("{}/trace.log", dir_path))?;

    #[cfg(feature = "time")]
    let replacement_file_appender = log4rs::append::file::FileAppender::builder()
        .encoder(Box::new(log4rs::encode::pattern::PatternEncoder::new(
            "{d} - {l} - {m}{n}",
        )))
        .build(&format!("{}/replacement.log", dir_path))?;

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
