use local_mixing::{
    circuit::{
        cf::Base2GateControlFunc,
        circuit::{par_check_equiv_probabilistic, Circuit},
        Gate,
    },
    compression::ct::CompressionTable,
    local_mixing::{
        test_search::test_local_mixing_search,
        tracer::{ReplacementFails, ReplacementSamples},
        LocalMixingJob,
    },
    replacement::{is_weakly_connected, strategy::ControlFnChoice},
};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use serde_json::json;
use std::collections::{HashMap, HashSet};
use std::env::args;
use std::fs::File;
use std::io::Write;

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

            Circuit::random_with_cf(
                num_wires,
                num_gates,
                ControlFnChoice::All,
                &mut ChaCha8Rng::from_os_rng(),
            )
            .save_as_json(&save_path);
            println!("Random circuit generated and saved to {}", save_path);
        }
        "local-mixing" => {
            let job_dir = args.next().expect("Missing job directory");
            let mut job = LocalMixingJob::load(&job_dir).expect("Failed to load job");
            job.run();
        }
        "search-test" => {
            let test_dir = args.next().expect("Missing test directory");
            test_local_mixing_search(&test_dir);
        }
        "build-compression-table" => {
            let save_path = args.next().expect("Missing compression table save path");
            let raw_cf_choice = args.next().expect("Missing cf choice");
            let cf_choice = ControlFnChoice::from_str(&raw_cf_choice).unwrap_or_else(|e| {
                panic!("Failed to parse cf choice: {}", e);
            });
            let max_gates_supported: usize = args
                .next()
                .expect("Missing number of gates suppported")
                .parse()
                .expect("Invalid num gates");
            let max_wires_supported: usize = args
                .next()
                .expect("Missing number of wires suppported")
                .parse()
                .expect("Invalid num wires");

            let ct = CompressionTable::new(max_gates_supported, max_wires_supported, cf_choice);
            ct.save_to_file(&save_path);
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

            let res = par_check_equiv_probabilistic(
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
        "distinguisher" => {
            // cargo run distinguisher <circuit_one_path> <circuit_two_path> <num_inputs> <save_json_path>
            let circuit_one_path = args.next().unwrap();
            let circuit_two_path = args.next().unwrap();
            let num_inputs = args.next().unwrap().parse().unwrap();
            let save_path = args.next().unwrap();
            let circuit_one = Circuit::load_from_json(&circuit_one_path);
            let circuit_two = Circuit::load_from_json(&circuit_two_path);
            let mut file = File::create(save_path).expect("Failed to create save file");

            assert_eq!(
                circuit_one.num_wires, circuit_two.num_wires,
                "Circuits have different sets of wires"
            );

            let mut rng = rand::rng();
            let mut results = HashMap::new();

            (0..num_inputs)
                .map(|_| {
                    (0..circuit_one.num_wires)
                        .map(|_| rng.random_bool(0.5))
                        .collect::<Vec<bool>>()
                })
                .for_each(|input| {
                    let evolution_one = circuit_one.evaluate_evolution(&input);
                    let evolution_two = circuit_two.evaluate_evolution(&input);

                    assert_eq!(
                        evolution_one.last(),
                        evolution_two.last(),
                        "Final states of the circuits do not match"
                    );

                    let hamming_weights_one: Vec<usize> = evolution_one
                        .iter()
                        .map(|state| state.iter().filter(|&&bit| bit).count())
                        .collect();

                    let hamming_weights_two: Vec<usize> = evolution_two
                        .iter()
                        .map(|state| state.iter().filter(|&&bit| bit).count())
                        .collect();

                    let hamming_distances: Vec<usize> = evolution_one
                        .iter()
                        .zip(evolution_two.iter())
                        .map(|(state_one, state_two)| {
                            state_one
                                .iter()
                                .zip(state_two.iter())
                                .filter(|(&bit_one, &bit_two)| bit_one != bit_two)
                                .count()
                        })
                        .collect();

                    let input_binary: String = input
                        .iter()
                        .map(|&bit| if bit { '1' } else { '0' })
                        .collect();
                    results.insert(
                        input_binary,
                        (hamming_weights_one, hamming_weights_two, hamming_distances),
                    );
                });

            let output_json = json!({
                "circuit-one": circuit_one_path,
                "circuit-two": circuit_two_path,
                "results": results
            });

            file.write_all(output_json.to_string().as_bytes())
                .expect("Failed to write to output file");
        }
        "analyze-replacements" => {
            let logs_path = args.next().expect("Missing /logs path");
            let repl_sample_path = format!("{}/replacement_samples.json", logs_path);
            let repl_fails_path = format!("{}/replacement_fails.json", logs_path);
            let replacement_samples: ReplacementSamples =
                serde_json::from_slice(&std::fs::read(repl_sample_path).unwrap()).unwrap();
            let replacement_fails: ReplacementFails =
                serde_json::from_slice(&std::fs::read(repl_fails_path).unwrap()).unwrap();

            for (i, inf_stage_replacement) in
                replacement_samples.inflationary_stage.iter().enumerate()
            {
                let c_original = Circuit {
                    num_wires: 64,
                    gates: inf_stage_replacement
                        .c_out
                        .iter()
                        .map(|&g| Gate::from(g))
                        .collect(),
                };
                let c_replacement = Circuit {
                    num_wires: 64,
                    gates: inf_stage_replacement
                        .c_in
                        .iter()
                        .map(|&g| Gate::from(g))
                        .collect(),
                };

                // Get # distinct targets
                let mut target_wires = HashSet::new();
                c_original.gates.iter().for_each(|g| {
                    target_wires.insert(g.wires[0]);
                });

                let c_original_wc = is_weakly_connected::<2>(&c_original.gates);
                let c_replacement_wc = is_weakly_connected::<4>(&c_replacement.gates);

                println!("Inflationary sample {}:", i);
                println!("# target wires: {}", target_wires.len());
                println!("c_original weakly-connected: {}", c_original_wc);
                println!("c_replacement weakly-connected: {}", c_replacement_wc);
                println!("c_original:");
                println!("{}\n", c_original.to_string());
                println!("c_replacement:");
                println!("{}\n", c_replacement.to_string());
            }

            for (i, knd_stage_replacement) in replacement_samples.kneading_stage.iter().enumerate()
            {
                let c_original = Circuit {
                    num_wires: 64,
                    gates: knd_stage_replacement
                        .c_out
                        .iter()
                        .map(|&g| Gate::from(g))
                        .collect(),
                };
                let c_replacement = Circuit {
                    num_wires: 64,
                    gates: knd_stage_replacement
                        .c_in
                        .iter()
                        .map(|&g| Gate::from(g))
                        .collect(),
                };

                // Get # distinct targets
                let mut target_wires = HashSet::new();
                c_original.gates.iter().for_each(|g| {
                    target_wires.insert(g.wires[0]);
                });

                let c_original_wc = is_weakly_connected::<4>(&c_original.gates);
                let c_replacement_wc = is_weakly_connected::<4>(&c_replacement.gates);

                println!("Kneading sample {}:", i);
                println!("# target wires: {}", target_wires.len());
                println!("c_original weakly-connected: {}", c_original_wc);
                println!("c_replacement weakly-connected: {}", c_replacement_wc);
                println!("c_original:");
                println!("{}\n", c_original.to_string());
                println!("c_replacement:");
                println!("{}\n", c_replacement.to_string());
            }

            for (i, knd_stage_fails) in replacement_fails.kneading_stage.iter().enumerate() {
                let c_original = Circuit {
                    num_wires: 64,
                    gates: knd_stage_fails
                        .c_out
                        .iter()
                        .map(|&g| Gate::from(g))
                        .collect(),
                };

                // Get # distinct targets
                let mut target_wires = HashSet::new();
                c_original.gates.iter().for_each(|g| {
                    target_wires.insert(g.wires[0]);
                });

                let c_original_wc = is_weakly_connected::<4>(&c_original.gates);

                println!("Failed kneading sample {}:", i);
                println!("# target wires: {}", target_wires.len());
                println!("c_original weakly-connected: {}", c_original_wc);
                println!("c_original:");
                println!("{}\n", c_original.to_string());
            }
        }
        _ => {
            eprintln!("Unknown command: {}", cmd);
        }
    }
}
