use std::{env::args, error::Error};

use local_mixing::{
    circuit::Circuit,
    local_mixing::{LocalMixingConfig, LocalMixingJob},
};
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

const DEFAULT_NUM_GATES: usize = 1000;

fn main() {
    // Setup logs
    let log_path = args().nth(1).expect("Missing log path");
    init_logs(&log_path).expect("Error initializing logs");

    // Load job config
    let config_path = args().nth(2).expect("Missing config path");
    let local_mixing_config =
        LocalMixingConfig::load_from_file(&config_path).expect("Error loading config file");

    // Store job destination path
    let job_destination_path = args().nth(3).expect("Missing destination path");

    // Rng
    let mut rng = ChaCha8Rng::from_entropy();

    // Load input circuit
    let input_circuit = match args().nth(4) {
        Some(path) => Circuit::load_from_file(&path).expect("Error loading config file"),
        None => {
            let ckt = Circuit::random(local_mixing_config.num_wires, DEFAULT_NUM_GATES, &mut rng);
            ckt.save_to_file("circuit.bin");
            log::info!("Randomly generated circuit saved to circuit.bin");
            ckt
        }
    };

    let mut obfuscation_job = LocalMixingJob::build(
        input_circuit,
        local_mixing_config,
        Some(job_destination_path),
    );
    
    run_strategy(&mut obfuscation_job);
}

fn init_logs(log_path: &str) -> Result<(), Box<dyn Error>> {
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

    log4rs::init_config(config)?;

    Ok(())
}

pub fn run_strategy(job: &mut LocalMixingJob) {
    let mut rng = ChaCha8Rng::from_entropy();

    let job_path = args().nth(2).expect("Missing job path");

    while job.curr_inflationary_step <= job.config.num_inflationary_steps {
        job.run_inflationary_step(&mut rng);

        if job.curr_inflationary_step % job.config.epoch_size == 0 {
            std::fs::write(&job_path, bincode::serialize(&job).unwrap()).unwrap();
        }
    }

    // while job.curr_kneading_step <= job.config.num_kneading_steps
    // // && job.curr_kneading_fail <= job.config.num_kneading_to_fail
    // {
    //     job.run_kneading_step(&mut rng);

    //     if job.curr_kneading_step % job.config.epoch_size == 0 {
    //         std::fs::write(&job_path, serde_json::to_string(&job).unwrap()).unwrap();
    //     }
    // }
}
