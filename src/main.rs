use local_mixing::{
    circuit::{
        analysis::num_distinct_wires,
        cf::{GateControlFunc, GateLibrary},
        circuit::{par_check_equiv_probabilistic, Circuit},
        Gate,
    },
    compression::{compress::compress, ct::CompressionTable, inflate_gate},
    local_mixing::{
        classify_replacements::{classify_fail, classify_success, SuccessCase},
        test_search::test_local_mixing_search,
        tracer::{ReplacementStatus, Tracer},
        LocalMixingJob,
    },
    replacement::is_weakly_connected,
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
            let gate_library = GateLibrary::from_str(&args.next().expect("Missing gate library"))
                .expect("Invalid gate library");

            Circuit::random_with_cf(
                num_wires,
                num_gates,
                gate_library,
                &mut ChaCha8Rng::from_os_rng(),
            )
            .save_as_json(&save_path);
            println!("Random circuit generated and saved to {}", save_path);
        }
        "random-inflated" => match args.next() {
            Some(save_dir) => {
                random_inflated(save_dir);
            }
            None => random_inflated("".to_string()),
        },
        "local-mixing" => {
            let job_dir = args.next().expect("Missing job directory");
            let mut job = LocalMixingJob::load(&job_dir).expect("Failed to load job");
            job.run();
        }
        "experiment" => {
            // Make sure <job>/inflationary directory already has input.json and config.json
            let job_dir = args.next().expect("Missing job directory");
            let inf_dir = job_dir.clone() + "/inflationary";
            let mut inflationary_job = LocalMixingJob::load(&inf_dir).expect("Failed to load job");
            inflationary_job.run();
            run_distinguisher(
                inf_dir.clone() + "/input.json",
                inf_dir.clone() + "/target.json",
                1000,
                inf_dir + "/input-target-plot.json",
            );
            let mut latest = inflationary_job.results();

            let mut iter = 0;
            loop {
                let knd_dir = job_dir.clone() + "/kneading/" + &iter.to_string();
                std::fs::create_dir(&knd_dir).unwrap_or_else(|error| panic!("{error:?}"));
                std::fs::create_dir(&(knd_dir.clone() + "/logs"))
                    .unwrap_or_else(|error| panic!("{error:?}"));
                latest.0.save_as_json(knd_dir.clone() + "/input.json");
                let mut knd_job = LocalMixingJob::experiment_config(&knd_dir, latest.0, latest.1);
                knd_job
                    .save_config_json(&(knd_dir.clone() + "/config.json"))
                    .unwrap_or_else(|error| panic!("{error:?}"));
                knd_job.run();
                run_distinguisher(
                    knd_dir.clone() + "/input.json",
                    knd_dir.clone() + "/target.json",
                    1000,
                    knd_dir + "/input-target-plot.json",
                );
                latest = knd_job.results();
                iter += 1;
            }
        }
        "search-test" => {
            let test_dir = args.next().expect("Missing test directory");
            test_local_mixing_search(&test_dir);
        }
        "build-compression-table" => {
            let save_path = args.next().expect("Missing compression table save path");
            let raw_gate_library = args.next().expect("Missing cf choice");
            let gate_library = GateLibrary::from_str(&raw_gate_library).unwrap_or_else(|e| {
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

            let ct = CompressionTable::new(max_gates_supported, max_wires_supported, gate_library);
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
            run_stats(circuit_path);
        }
        "compress" => {
            let circuit_path = args.next().unwrap();
            compress(circuit_path);
        }
        "distinguisher" => {
            let circuit_one_path = args.next().unwrap();
            let circuit_two_path = args.next().unwrap();
            let num_inputs = args.next().unwrap().parse().unwrap();
            let save_path = args.next().unwrap();
            run_distinguisher(circuit_one_path, circuit_two_path, num_inputs, save_path);
        }
        "analyze-replacements" => {
            let repl_sample_path = args.next().expect("Missing trace.json path");
            analyze_replacements(repl_sample_path);
        }
        "display" => {
            let circuit_path = args.next().unwrap();
            println!(
                "{}",
                Circuit::load_from_json(&circuit_path).to_string_vertical()
            );
        }
        _ => {
            eprintln!("Unknown command: {}", cmd);
        }
    }
}

fn random_inflated(path: String) {
    let identity_compenents: (Vec<Vec<Gate>>, HashMap<Vec<usize>, Vec<Vec<Gate>>>) =
        bincode::deserialize_from(
            std::fs::File::open("bin/4-gate-3-wire-TwoBit-optimal-halves.bin")
                .expect("Failed to open bin/id_table.db"),
        )
        .expect("Failed to deserialize id_table");
    let ct = CompressionTable::from_file("bin/table-twobit.db");

    let num_wires = 16;
    let num_gates = 256;
    let gate_library = GateLibrary::TwoBit;

    let mut rng = rand::rng();
    let original = Circuit::random_with_cf(num_wires, num_gates, gate_library, &mut rng);
    let mut new_gates: Vec<Gate> = vec![];

    for i in 0..original.gates.len() {
        let inflated = inflate_gate(
            &original.gates[i],
            &identity_compenents.0,
            3,
            &identity_compenents.1,
            &ct,
            &mut rng,
        );
        new_gates.extend(inflated);
    }

    let input = Circuit {
        num_wires,
        gates: new_gates,
    };

    original.save_as_json(path.clone() + "/original.json");
    input.save_as_json(path + "/input.json");
}

fn run_stats(circuit_path: String) {
    let circuit = Circuit::load_from_json(circuit_path);

    let mut cf_freq = [0u32; GateControlFunc::COUNT as usize];
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

fn run_distinguisher(
    circuit_one_path: String,
    circuit_two_path: String,
    num_inputs: usize,
    save_path: String,
) {
    let circuit_one = Circuit::load_from_json(&circuit_one_path);
    let circuit_two = Circuit::load_from_json(&circuit_two_path);
    let mut file = File::create(save_path).expect("Failed to create save file");

    assert_eq!(
        circuit_one.num_wires, circuit_two.num_wires,
        "Circuits have different sets of wires"
    );

    let circuit_one_len = circuit_one.gates.len();
    let circuit_two_len = circuit_two.gates.len();

    let mut rng = rand::rng();
    let mut results = HashMap::new();
    for i1 in 0..circuit_one_len + 1 {
        for i2 in 0..circuit_two_len + 1 {
            results.insert((i1, i2), 0 as f64);
        }
    }

    (0..num_inputs)
        .map(|_| {
            (0..circuit_one.num_wires)
                .map(|_| rng.random_bool(0.5))
                .collect::<Vec<bool>>()
        })
        .for_each(|input| {
            let evolution_one = circuit_one.evaluate_evolution(&input);
            let evolution_two = circuit_two.evaluate_evolution(&input);

            for i1 in 0..circuit_one_len + 1 {
                for i2 in 0..circuit_two_len + 1 {
                    let hamming_dist = evolution_one[i1]
                        .iter()
                        .zip(evolution_two[i2].iter())
                        .filter(|(&b1, &b2)| b1 != b2)
                        .count();
                    let overlap = (2 * hamming_dist) as f64 / circuit_one.num_wires as f64 - 1.0;
                    let abs_overlap = overlap.abs();
                    results.entry((i1, i2)).and_modify(|o| *o += abs_overlap);
                }
            }
        });

    let results_as_vector: Vec<[f64; 3]> = results
        .into_iter()
        .map(|((i1, i2), value)| [i1 as f64, i2 as f64, value / num_inputs as f64])
        .collect();

    let output_json = json!({
        "circuit-one-len": circuit_one_len,
        "circuit-two-len": circuit_two_len,
        "results": results_as_vector
    });

    file.write_all(output_json.to_string().as_bytes())
        .expect("Failed to write to output file");
}

fn analyze_replacements(repl_sample_path: String) {
    let trace_data: Tracer =
        serde_json::from_slice(&std::fs::read(repl_sample_path).unwrap()).unwrap();

    let mut success_cases = vec![];
    let mut fail_cases = vec![];
    trace_data
        .kneading_stage
        .iter()
        .for_each(|step| match &step.replacement_fields.data {
            ReplacementStatus::Success(input, output) => {
                success_cases.push((step.current_step, input.clone(), output.clone()));
            }
            ReplacementStatus::Fail(circuit) => {
                fail_cases.push((step.current_step, circuit.clone()));
            }
        });

    success_cases.sort_by_key(|sample| sample.0);
    fail_cases.sort_by_key(|sample| sample.0);

    for (i, inf_stage_replacement) in trace_data.inflationary_stage.iter().enumerate() {
        match &inf_stage_replacement.replacement_fields.data {
            ReplacementStatus::Success(input, output) => {
                let input = Circuit {
                    num_wires: 64,
                    gates: input.iter().map(|&g| Gate::from(g)).collect(),
                };
                let output = Circuit {
                    num_wires: 64,
                    gates: output.iter().map(|&g| Gate::from(g)).collect(),
                };

                println!("Inflationary sample {}:", i);
                println!(
                    "input # distinct wires: {}",
                    num_distinct_wires(&input.gates)
                );
                println!(
                    "output # distinct wires: {}",
                    num_distinct_wires(&output.gates)
                );
                println!("input:");
                println!("{}\n", input.to_string());
                println!("output:");
                println!("{}\n", output.to_string());
            }
            ReplacementStatus::Fail(_) => todo!(),
        }
    }

    for (i, knd_stage_replacement) in success_cases.iter().enumerate() {
        let input = &knd_stage_replacement.1;
        let output = &knd_stage_replacement.2;
        let classification = classify_success(input, output);

        let input = Circuit {
            num_wires: 64,
            gates: input.iter().map(|&g| Gate::from(g)).collect(),
        };
        let output = Circuit {
            num_wires: 64,
            gates: output.iter().map(|&g| Gate::from(g)).collect(),
        };

        // Get # distinct targets
        let mut target_wires = HashSet::new();
        input.gates.iter().for_each(|g| {
            target_wires.insert(g.wires[0]);
        });

        let input_wc = is_weakly_connected(&input.gates);
        let output_wc = is_weakly_connected(&output.gates);

        println!("Kneading sample {}:", i);
        println!("# target wires: {}", target_wires.len());
        println!("input weakly-connected: {}", input_wc);
        println!("output weakly-connected: {}", output_wc);
        println!("type: {:?}", classification);
        println!("input:");
        println!("{}\n", input.to_string());
        println!("output:");
        println!("{}\n", output.to_string());
    }

    for (i, knd_stage_fails) in fail_cases.iter().enumerate() {
        let circuit = &knd_stage_fails.1;
        let classification = classify_fail(circuit);
        let input = Circuit {
            num_wires: 64,
            gates: circuit.iter().map(|&g| Gate::from(g)).collect(),
        };

        // Get # distinct targets
        let mut target_wires = HashSet::new();
        input.gates.iter().for_each(|g| {
            target_wires.insert(g.wires[0]);
        });

        let input_wc = is_weakly_connected(&input.gates);

        println!("Failed kneading sample {}:", i);
        println!("# target wires: {}", target_wires.len());
        println!("input weakly-connected: {}", input_wc);
        println!("type: {:?}", classification);
        println!("input:");
        println!("{}\n", input.to_string());
    }

    println!("Success type frequencies over time:");
    let mut success_freq = vec![0; SuccessCase::COUNT];
    for (i, knd_stage_replacement) in success_cases.iter().enumerate() {
        let input = &knd_stage_replacement.1;
        let output = &knd_stage_replacement.2;
        let classification = classify_success(input, output);

        if classification.contains(&SuccessCase::IdentitySubcircuits) {
            success_freq[0] += 1;
        }
        if classification.contains(&SuccessCase::NegatedCFs) {
            success_freq[1] += 1;
        }
        if classification.contains(&SuccessCase::Other) {
            success_freq[2] += 1;
        }

        println!(
            "sample/snapshot {}: IdentitySubcircuits: {}, NegatedCFs: {}, Other: {}",
            i, success_freq[0], success_freq[1], success_freq[2]
        );
    }
}
