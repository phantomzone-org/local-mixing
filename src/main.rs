use std::{env::args, error::Error};

use local_mixing::{
    circuit::Circuit,
    local_mixing::{LocalMixingConfig, LocalMixingJob},
};
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

pub fn run_strategy(job: &mut LocalMixingJob) {
    // setup logs
    let log_path = args().nth(1).expect("Missing log path");
    let log_confg = create_log4rs_config(&log_path).unwrap();
    log4rs::init_config(log_confg).unwrap();

    let orignal_job_path = args().nth(2).expect("Missing original circuit path");
    std::fs::write(
        orignal_job_path,
        serde_json::to_string(&job.config).unwrap(),
    )
    .unwrap();

    let mut rng = ChaCha8Rng::from_entropy();

    while job.curr_inflationary_step <= job.config.num_inflationary_steps {
        job.run_inflationary_step(&mut rng);
    }

    while job.curr_kneading_step <= job.config.num_kneading_steps
        && job.curr_kneading_fail <= job.config.num_kneading_to_fail
    {
        job.run_kneading_step(&mut rng);
    }
}

fn main() {
    let num_wires = 64;
    let num_gates = 1000000;
    let mut rng = ChaCha8Rng::from_entropy();

    let random_circuit = Circuit::random(num_wires, num_gates, &mut rng);
    let config = LocalMixingConfig {
        original_circuit: random_circuit,
        num_wires,
        num_inflationary_steps: 300000,
        num_kneading_steps: 300000,
        num_replacement_attempts: 10000000,
        num_inflationary_to_fail: 10000,
        num_kneading_to_fail: 10000,
    };
    let mut job = LocalMixingJob::new(config);

    run_strategy(&mut job);
}

fn create_log4rs_config(log_path: &str) -> Result<log4rs::Config, Box<dyn Error>> {
    // Define the file appender with the specified path and pattern
    let file_appender = log4rs::append::file::FileAppender::builder()
        .encoder(Box::new(log4rs::encode::pattern::PatternEncoder::new(
            "{d} - {l} - {m}{n}",
        )))
        .build(log_path)?;

    // Build the configuration
    let config = log4rs::Config::builder()
        .appender(log4rs::config::Appender::builder().build("file", Box::new(file_appender)))
        .build(
            log4rs::config::Root::builder()
                .appender("file")
                .build(log::LevelFilter::Trace),
        )?;

    Ok(config)
}
