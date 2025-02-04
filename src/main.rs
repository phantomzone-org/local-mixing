use local_mixing::{circuit::Circuit, local_mixing::LocalMixingJob};
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use std::{env::args, error::Error};

fn main() {
    let mut args = args();
    let _ = args.next();
    let cmd = args.next().expect("Missing command");

    match cmd.as_str() {
        "random-circuit" => {
            let save_path = args.next().expect("Missing circuit path");
            let num_wires: u32 = args
                .next()
                .expect("Missing number of wires")
                .parse()
                .expect("Invalid number of wires");
            let num_gates: usize = args
                .next()
                .expect("Missing number of gates")
                .parse()
                .expect("Invalid number of gates");

            Circuit::random(num_wires, num_gates, &mut ChaCha8Rng::from_entropy())
                .save_as_binary(&save_path);
            println!("Random circuit generated and saved to {}", save_path);
        }
        "json" => {
            let circuit_path = args.next().expect("Missing circuit path");

            let circuit = Circuit::load_from_binary(&circuit_path);

            if let Some(json_path) = args.next() {
                circuit.save_as_json(&json_path);
                println!("Circuit JSON saved to {}", json_path);
            } else {
                println!("{:#?}", circuit);
            }
        }
        "local-mixing" => {
            let log_path = args.next().expect("Missing log path");
            let config_path = args.next().expect("Missing config path");

            init_logs(&log_path).expect("Error initializing logs");

            let success = LocalMixingJob::load(config_path).execute();
            let status = if success { "SUCCESS" } else { "FAIL" };
            log::info!("Local mixing finished, status = {}", status);
        }
        _ => {
            eprintln!("Unknown command: {}", cmd);
        }
    }
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
