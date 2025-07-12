use local_mixing::{
    circuit::{
        analysis::num_distinct_wires,
        cf::{GateControlFunc, GateLibrary},
        circuit::{check_equiv_probabilistic, par_check_equiv_probabilistic, Circuit},
        Gate,
    },
    compression::{
        compress::compress, ct::CompressionTable, inflate_gate, inflate_gate_to_block,
        IncompressibleCircuitsTable,
    },
    local_mixing::{
        classify_replacements::{classify_fail, classify_success},
        search::{find_convex_gate_ids_max_spread_overall, permute_circuit},
        test_search::test_local_mixing_search,
        tracer::{ReplacementStatus, Tracer},
        LocalMixingJob,
    },
    replacement::{find_replacement_random_sample, is_weakly_connected},
};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use serde_json::json;
use std::collections::HashSet;
use std::env::args;
use std::fs::File;
use std::io::Write;
use std::time::Instant;

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
        "random-inflated-block" => match args.next() {
            Some(save_dir) => {
                random_inflated_block(save_dir);
            }
            None => random_inflated_block("".to_string()),
        },
        "mix-identity" => {
            let job_dir = args.next().expect("Missing job directory");
            let input = Circuit::load_from_json(format!("{}/input.json", job_dir.clone()));
            let mut circuit = input.clone();
            let mut rng = ChaCha8Rng::from_os_rng();

            for i in 0..100 {
                println!("{}, {} gates", i, circuit.gates.len());
                let splice_index = rng.random_range(0..circuit.gates.len());
                let mut new_gates = circuit.gates[splice_index..].to_vec();
                new_gates.extend(&circuit.gates[..splice_index]);
                circuit.gates = new_gates;

                for _ in 0..100 {
                    let (selected_gate_idx, _) = find_convex_gate_ids_max_spread_overall(
                        10,
                        2,
                        100,
                        circuit.num_wires,
                        &circuit.gates,
                        &mut rng,
                    );
                    let repl_in: Vec<_> = selected_gate_idx
                        .iter()
                        .map(|&i| circuit.gates[i])
                        .collect();
                    if let Some((repl, _)) = find_replacement_random_sample(
                        &repl_in,
                        circuit.num_wires,
                        4,
                        1000000,
                        GateLibrary::TwoBit,
                        false,
                        &mut rng,
                    ) {
                        let c_out_start = permute_circuit(
                            circuit.num_wires,
                            &mut circuit.gates,
                            &selected_gate_idx,
                        );
                        circuit.gates.splice(c_out_start..c_out_start + 2, repl);
                    }
                }
            }

            assert!(check_equiv_probabilistic(
                input.num_wires,
                &input.gates,
                &circuit.gates,
                10000,
                &mut rng
            )
            .is_ok());

            circuit.save_as_json("output.json");
        }
        "local-mixing" => {
            let job_dir = args.next().expect("Missing job directory");
            let mut job = LocalMixingJob::load(&job_dir).expect("Failed to load job");
            job.run();
        }
        "shuffle" => {
            let job_dir = args.next().expect("Missing directory");
            let ckt_path = job_dir.clone() + "/input.json";
            let c = Circuit::load_from_json(ckt_path);
            let mut rng = rand::rng();

            let mut indegrees = vec![0; c.gates.len()];
            for i in 0..c.gates.len() {
                for j in i + 1..c.gates.len() {
                    if c.gates[i].collides_with(&c.gates[j]) {
                        indegrees[j] += 1;
                    }
                }
            }
            println!("indegrees computed");

            let mut out: Vec<Gate> = Vec::with_capacity(c.gates.len());
            let mut available: Vec<usize> = (0..c.gates.len()).collect();

            while available.len() > 0 {
                println!("gates left to insert: {}", available.len());
                // Find all available gates with indegree 0
                let zero_indegree: Vec<usize> = available
                    .iter()
                    .cloned()
                    .filter(|&idx| indegrees[idx] == 0)
                    .collect();

                assert!(
                    !zero_indegree.is_empty(),
                    "No available gate with indegree 0, possible cycle"
                );

                // Pick a random gate from zero_indegree
                let pick_idx = rng.random_range(0..zero_indegree.len());
                let gate_idx = zero_indegree[pick_idx];

                // Push the gate to output
                out.push(c.gates[gate_idx].clone());

                // Remove the gate from available
                available.retain(|&x| x != gate_idx);

                // Decrement indegrees for gates that collide with this gate and are still available
                for &other_idx in &available {
                    if c.gates[gate_idx].collides_with(&c.gates[other_idx]) {
                        indegrees[other_idx] -= 1;
                    }
                }
            }

            let target = Circuit {
                num_wires: c.num_wires,
                gates: out,
            };
            target.save_as_json(job_dir + "/target.json");
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
    let identity_compenents =
        IncompressibleCircuitsTable::from_file("bin/4-gate-4-wire-TwoBit-optimal-halves.bin");
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
            num_wires,
            &identity_compenents,
            &ct,
            &mut rng,
        );
        new_gates.extend(inflated);
    }

    let input = Circuit {
        num_wires,
        gates: new_gates,
    };

    if path == "" {
        original.save_as_json("original.json");
        input.save_as_json("input.json");
    } else {
        original.save_as_json(path.clone() + "/original.json");
        input.save_as_json(path + "/input.json");
    }
}

fn random_inflated_block(path: String) {
    let identity_compenents =
        IncompressibleCircuitsTable::from_file("bin/4-gate-4-wire-TwoBit-optimal-halves.bin");
    let ct = CompressionTable::from_file("bin/table-twobit.db");

    let num_wires = 16;
    let num_gates = 256;
    let gate_library = GateLibrary::TwoBit;

    let mut rng = rand::rng();
    let original = Circuit::random_with_cf(num_wires, num_gates, gate_library, &mut rng);
    let mut new_gates: Vec<Gate> = vec![];

    for i in 0..original.gates.len() {
        let inflated = inflate_gate_to_block(
            &original.gates[i],
            num_wires,
            &identity_compenents,
            &ct,
            &mut rng,
        );
        new_gates.extend(inflated);
    }

    let input = Circuit {
        num_wires,
        gates: new_gates,
    };

    if path == "" {
        original.save_as_json("original.json");
        input.save_as_json("input.json");
    } else {
        original.save_as_json(path.clone() + "/original.json");
        input.save_as_json(path + "/input.json");
    }
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

    println!("Bitline influence:");
    let mut num_gates_touching_wires = vec![0; circuit.num_wires];
    let mut num_targets_touching_wires = vec![0; circuit.num_wires];
    let mut num_controls_touching_wires = vec![0; circuit.num_wires];
    for g in circuit.gates {
        num_gates_touching_wires[g.wires[0]] += 1;
        num_targets_touching_wires[g.wires[0]] += 1;

        num_gates_touching_wires[g.wires[1]] += 1;
        num_controls_touching_wires[g.wires[1]] += 1;
        num_gates_touching_wires[g.wires[2]] += 1;
        num_controls_touching_wires[g.wires[2]] += 1;
    }
    dbg!(num_gates_touching_wires);
    dbg!(num_targets_touching_wires);
    dbg!(num_controls_touching_wires);
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
    let num_wires = circuit_one.num_wires;

    let s = Instant::now();
    let mut average =
        vec![[0 as f64, 0 as f64, 0.0]; (circuit_one_len + 1) * (circuit_two_len + 1)];

    for i in 0..num_inputs {
        println!("{}/{}", i, num_inputs);
        let input: Vec<bool> = (0..num_wires).map(|_| rng.random_bool(0.5)).collect();

        let evolution_one = circuit_one.evaluate_evolution(&input);
        let evolution_two = circuit_two.evaluate_evolution(&input);

        for i1 in 0..=circuit_one_len {
            for i2 in 0..=circuit_two_len {
                let hamming_dist = evolution_one[i1]
                    .iter()
                    .zip(evolution_two[i2].iter())
                    .filter(|(&b1, &b2)| b1 != b2)
                    .count();
                let overlap = (2 * hamming_dist) as f64 / num_wires as f64 - 1.0;
                let abs_overlap = overlap.abs();
                let index = i1 * (circuit_two_len + 1) + i2;
                average[index][0] = i1 as f64;
                average[index][1] = i2 as f64;
                average[index][2] += abs_overlap / num_inputs as f64;
            }
        }
    }

    let d = Instant::now() - s;
    println!("time: {:?}", d);

    let output_json = json!({
        "circuit-one-len": circuit_one_len,
        "circuit-two-len": circuit_two_len,
        "results": average,
    });

    file.write_all(output_json.to_string().as_bytes())
        .expect("Failed to write to output file");
}

fn analyze_replacements(repl_sample_path: String) {
    let trace_data: Tracer =
        serde_json::from_slice(&std::fs::read(repl_sample_path).unwrap()).unwrap();

    // Kneading stage: separate and sort
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

    // Inflationary stage: separate and sort
    let mut inf_success_cases = vec![];
    let mut inf_fail_cases = vec![];
    trace_data
        .inflationary_stage
        .iter()
        .for_each(|step| match &step.replacement_fields.data {
            ReplacementStatus::Success(input, output) => {
                inf_success_cases.push((step.current_step, input.clone(), output.clone()));
            }
            ReplacementStatus::Fail(circuit) => {
                inf_fail_cases.push((step.current_step, circuit.clone()));
            }
        });

    inf_success_cases.sort_by_key(|sample| sample.0);
    inf_fail_cases.sort_by_key(|sample| sample.0);

    // Print inflationary successes
    for (i, inf_stage_replacement) in inf_success_cases.iter().enumerate() {
        let input = Circuit {
            num_wires: 64,
            gates: inf_stage_replacement
                .1
                .iter()
                .map(|&g| Gate::from(g))
                .collect(),
        };
        let output = Circuit {
            num_wires: 64,
            gates: inf_stage_replacement
                .2
                .iter()
                .map(|&g| Gate::from(g))
                .collect(),
        };

        // Get # distinct targets
        let mut target_wires = HashSet::new();
        input.gates.iter().for_each(|g| {
            target_wires.insert(g.wires[0]);
        });

        let input_wc = is_weakly_connected(&input.gates);
        let output_wc = is_weakly_connected(&output.gates);

        println!("Inflationary sample {}:", i);
        println!("# target wires: {}", target_wires.len());
        println!("input weakly-connected: {}", input_wc);
        println!("output weakly-connected: {}", output_wc);
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

    // Print inflationary fails
    for (i, inf_stage_fails) in inf_fail_cases.iter().enumerate() {
        let circuit = &inf_stage_fails.1;
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

        println!("Failed inflationary sample {}:", i);
        println!("# target wires: {}", target_wires.len());
        println!("input weakly-connected: {}", input_wc);
        println!("input:");
        println!("{}\n", input.to_string());
    }

    // Print kneading successes
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

    // Print kneading fails
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
}
