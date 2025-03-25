use local_mixing::{
    cc::run_compression_strategy_one,
    circuit::{
        cf::Base2GateControlFunc,
        circuit::{check_equiv_probabilistic, Circuit},
    },
    local_mixing::LocalMixingJob,
    replacement::{
        strategy::{ControlFnChoice, ReplacementStrategy},
        test::test_num_samples,
    },
};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use std::{env::args, error::Error};

fn main() {
    run();
}

fn run() {
    let mut args = args();
    let _ = args.next();
    let cmd = args.next().expect("Missing command");

    match cmd.as_str() {
        "random-circuit" => {
            let save_path = args.next().expect("Missing circuit path");
            let num_wires = args
                .next()
                .expect("Missing number of wires")
                .parse()
                .expect("Invalid number of wires");
            let num_gates: usize = args
                .next()
                .expect("Missing number of gates")
                .parse()
                .expect("Invalid number of gates");

            Circuit::random(num_wires, num_gates, &mut ChaCha8Rng::from_os_rng())
                .save_as_json(&save_path);
            println!("Random circuit generated and saved to {}", save_path);
        }
        "local-mixing" => {
            let job_dir = args.next().expect("Missing job directory");
            let mut job = LocalMixingJob::load(&job_dir).expect("Failed to load job");
            let _success = job.execute(&job_dir);
            #[cfg(feature = "trace")]
            {
                let status = if _success { "SUCCESS" } else { "FAIL" };
                log::info!(target: "trace", "Local mixing finished, status = {}", status);
            }
        }
        "json" => {
            let circuit_path = args.next().expect("Missing circuit path");

            let circuit = Circuit::load_from_json(&circuit_path);

            if let Some(json_path) = args.next() {
                circuit.save_as_json(&json_path);
                println!("Circuit JSON saved to {}", json_path);
            } else {
                println!("{:#?}", circuit);
            }
        }
        "replace" => {
            let log_path = args.next().expect("Missing log path");
            let strategy_u8 = args
                .next()
                .expect("Missing strategy")
                .parse()
                .expect("Invalid strategy input");
            let cf_choice_u8 = args
                .next()
                .expect("Missing control func choice")
                .parse()
                .expect("Invalid cf input");
            let n_iter = args
                .next()
                .expect("Missing n_iter")
                .parse()
                .expect("Invalid value for n_iter");
            let strategy =
                ReplacementStrategy::from_u8(strategy_u8).expect("Strategy does not exist");
            let cf_choice =
                ControlFnChoice::from_u8(cf_choice_u8).expect("ControlFnChoice does not exist");

            init_logs(&log_path).expect("Error initializing logs");

            test_num_samples(strategy, cf_choice, n_iter);
        }
        "equiv" => {
            let circuit_one_path = args.next().expect("Missing circuit 1 path");
            let circuit_two_path = args.next().expect("Missing circuit 2 path");
            let num_iter = args
                .next()
                .expect("Missing number of sample inputs")
                .parse()
                .expect("Invalid input");
            let circuit_one = Circuit::load_from_json(circuit_one_path);
            let circuit_two = Circuit::load_from_json(circuit_two_path);
            let mut rng = ChaCha8Rng::from_os_rng();

            let res = check_equiv_probabilistic(
                circuit_one.num_wires,
                &circuit_one.gates,
                &circuit_two.gates,
                num_iter,
                &mut rng,
            );
            match res {
                Ok(()) => println!("func equiv check passes"),
                Err(e) => println!("func equiv check fails: {}", e),
            }
        }
        "stats" => {
            let circuit_path = args.next().expect("Missing circuit path");
            let circuit = Circuit::load_from_json(circuit_path);

            let mut cf_freq = [0u32; Base2GateControlFunc::COUNT as usize];
            for g in &circuit.gates {
                cf_freq[g.control_func as usize] += 1;
            }

            println!("Control functions:");
            let total_gates = circuit.gates.len() as f32;
            for (i, &count) in cf_freq.iter().enumerate() {
                let proportion = count as f32 / total_gates;
                println!("{}: {:.2}%", i, proportion * 100.0);
            }
        }
        "compress" => {
            let circuit_path = args.next().expect("Missing circuit path");
            run_compression_strategy_one(&circuit_path);
        }
        "correlate" => {
            let circuit_one_path = args.next().unwrap();
            let circuit_two_path = args.next().unwrap();
            let circuit_one = Circuit::load_from_json(&circuit_one_path);
            let circuit_two = Circuit::load_from_json(&circuit_two_path);

            let mut rng = ChaCha8Rng::from_os_rng();
            let input: Vec<bool> = (0..circuit_one.num_wires)
                .map(|_| rng.random_bool(0.5))
                .collect();
            let ev_one = circuit_one.evaluate_evolution(&input);
            let ev_two = circuit_two.evaluate_evolution(&input);

            let max_len = ev_one.len().max(ev_two.len());
            for i in 0..max_len {
                let ev_one_str = ev_one.get(i).map_or_else(
                    || "N/A".to_string(),
                    |entry| entry.iter().map(|&b| if b { '1' } else { '0' }).collect(),
                );
                let ev_two_str = ev_two.get(i).map_or_else(
                    || "N/A".to_string(),
                    |entry| entry.iter().map(|&b| if b { '1' } else { '0' }).collect(),
                );
                println!(
                    "ev_one[{}]: {:<20} | ev_two[{}]: {}",
                    i, ev_one_str, i, ev_two_str
                );
            }

            for i in 0..ev_one.len() {
                let ev_one_str: String = ev_one[i]
                    .iter()
                    .map(|&b| if b { '1' } else { '0' })
                    .collect();
                let ev_two_str: String = ev_two[i]
                    .iter()
                    .map(|&b| if b { '1' } else { '0' })
                    .collect();

                let hamming_weight_one = ev_one[i].iter().filter(|&&b| b).count();
                let hamming_weight_two = ev_two[i].iter().filter(|&&b| b).count();

                let hamming_distance = ev_one[i]
                    .iter()
                    .zip(&ev_two[i])
                    .filter(|(&b1, &b2)| b1 != b2)
                    .count();

                println!(
                    "Index {}: ev_one: {}, Hamming weight: {}, ev_two: {}, Hamming weight: {}, Hamming distance: {}",
                    i, ev_one_str, hamming_weight_one, ev_two_str, hamming_weight_two, hamming_distance
                );
            }
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
