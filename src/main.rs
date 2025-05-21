use local_mixing::{
    circuit::{
        analysis::{num_distinct_wires, projection_circuit, truth_table},
        cf::{GateControlFunc, GateLibrary},
        circuit::{
            check_equiv_probabilistic, correct_controls, par_check_equiv_probabilistic, Circuit,
        },
        Gate,
    },
    compression::{compress::compress, ct::CompressionTable},
    local_mixing::{
        classify_replacements::{classify_fail, classify_success, SuccessCase},
        test_search::test_local_mixing_search,
        tracer::{ReplacementStatus, Tracer},
        LocalMixingJob,
    },
    replacement::{is_weakly_connected, replace_ct::find_replacement_with_ct},
};
use rand::{seq::IndexedRandom, Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use serde_json::json;
use std::collections::{HashMap, HashSet};
use std::env::args;
use std::fs::File;
use std::io::Write;

fn tmp2() {
    let mut rng = rand::rng();
    let original = Circuit::random_with_cf(16, 256, GateLibrary::TwoBit, &mut rng);
    let mut evolved = original.gates.clone();
    let ct = CompressionTable::from_file("bin/table-twobit.db");

    let num_iterations = 20;
    for iter in 0..num_iterations {
        println!("iter: {}", iter);
        let first_gate = evolved.remove(0);
        let mut i = 0;
        while i < evolved.len() {
            // println!("{} / {}", i, evolved.len());
            if evolved[i].collides_with(&first_gate) {
                let repl = vec![first_gate, evolved[i], first_gate];
                if let Some(_) =
                    ct.lookup_truth_table(&truth_table(9, &projection_circuit(&repl).0))
                {
                    let new_repl =
                        find_replacement_with_ct(&repl, 16, 5, 100, &ct, false, &mut rng).unwrap();
                    evolved.splice(i..i + 1, new_repl.0);

                    i += 5;
                } else {
                    panic!();
                }
            } else {
                i += 1;
            }
        }
        evolved.push(first_gate);

        let evolved_ckt = Circuit {
            num_wires: 16,
            gates: evolved.clone(),
        };
        evolved_ckt.save_as_json("evolved.json");
        original.save_as_json("original.json");
    }

    assert!(check_equiv_probabilistic(16, &original.gates, &evolved, 10000, &mut rng).is_ok());
}

fn tmp() {
    let wires = 16;
    let mut rng = rand::rng();
    let original = Circuit::random_with_cf(wires, 256, GateLibrary::TwoBit, &mut rng);
    let id_table: Vec<Vec<Gate>> = bincode::deserialize_from(
        std::fs::File::open("bin/id_table.db").expect("Failed to open bin/id_table.db"),
    )
    .expect("Failed to deserialize id_table");

    let mut input_gates = original.gates.clone();
    // for i in (0..input_gates.len()).rev() {
    for i in (1..input_gates.len()).rev() {
        let mut rand_gates = id_table.choose(&mut rng).unwrap().clone();
        // let mut new_wires = [0; 3];
        // let mut iter = 0;
        // while iter < 3 {
        //     let w = rng.random_range(0..wires);
        //     if !new_wires.contains(&w) {
        //         new_wires[iter] = w;
        //         iter += 1;
        //     }
        // }
        let mut new_wires = [0; 3];
        new_wires[0] = input_gates[i].wires[rng.random_range(0..3)];
        // new_wires[1] = input_gates[i - 1].wires[rng.random_range(0..3)];
        new_wires[1] = {
            loop {
                let w = input_gates[i - 1].wires[rng.random_range(0..3)];
                if w != new_wires[0] {
                    break w;
                }
            }
        };
        new_wires[2] = {
            loop {
                let w = rng.random_range(0..wires);
                if w != new_wires[0] && w != new_wires[1] {
                    break w;
                }
            }
        };

        rand_gates.iter_mut().for_each(|g| {
            g.wires[0] = new_wires[g.wires[0]];
            g.wires[1] = new_wires[g.wires[1]];
            g.wires[2] = new_wires[g.wires[2]];
        });

        input_gates.splice(i..i, rand_gates.clone());
    }

    correct_controls(&mut input_gates);

    let input = Circuit {
        num_wires: wires,
        gates: input_gates.clone(),
    };

    assert!(check_equiv_probabilistic(16, &original.gates, &input_gates, 10000, &mut rng).is_ok());

    original.save_as_json("original.json");
    input.save_as_json("input.json");
}

fn main() {
    // tmp();
    run();

    // let mut rng = rand::rng();
    // let ct = CompressionTable::from_file("bin/table-r57.db");
    // for _ in 0..100 {
    //     let c = Circuit::random_with_cf(5, 5, GateLibrary::R57, &mut rng).gates;
    //     let tt = truth_table(ct.max_wires_supported, &projection_circuit(&c).0);
    //     if let Some(cxity) = ct.lookup_truth_table(&tt) {
    //         println!("cxity = {}", cxity);
    //     } else {
    //         println!("cxity > {}", ct.max_gates_supported);
    //     }
    // }

    // let mut rng = rand::rng();
    // let original = Circuit::random_with_cf(16, 256, GateLibrary::TwoBit, &mut rng);
    // let id_table: Vec<Vec<Gate>> = bincode::deserialize_from(
    //     std::fs::File::open("bin/id_table.db").expect("Failed to open bin/id_table.db"),
    // )
    // .expect("Failed to deserialize id_table");
    // let mut new_gates: Vec<Gate> = Vec::with_capacity(original.gates.len() * 8);

    // for g in &original.gates {
    //     let g_proj = Gate {
    //         wires: [0, 1, 2],
    //         control_func: g.control_func,
    //         generation: 0,
    //     };
    //     loop {
    //         let replacement: Vec<Gate> = id_table.choose(&mut rng).unwrap().to_vec();
    //         if let Some(pos) = replacement.iter().position(|r| r.equal_to(&g_proj)) {
    //             let mut final_repl: Vec<Gate> = vec![];
    //             final_repl.extend(&replacement[pos + 1..]);
    //             final_repl.extend(&replacement[..pos]);
    //             final_repl.iter_mut().for_each(|r| {
    //                 r.wires[0] = g.wires[r.wires[0]];
    //                 r.wires[1] = g.wires[r.wires[1]];
    //                 r.wires[2] = g.wires[r.wires[2]];
    //             });
    //             new_gates.extend(final_repl);
    //             break;
    //         }
    //     }
    // }

    // let res = check_equiv_probabilistic(16, &original.gates, &new_gates, 10000, &mut rng);
    // if res.is_err() {
    //     dbg!(original.gates);
    //     dbg!(new_gates);
    //     panic!();
    // }

    // let input = Circuit {
    //     num_wires: 16,
    //     gates: new_gates,
    // };

    // input.save_as_json("input.json");
    // original.save_as_json("original.json");
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
                GateLibrary::All,
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
        "compress" => {
            let circuit_path = args.next().unwrap();
            compress(circuit_path);
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
                .map(|i| {
                    println!("iteration {}/{}", i + 1, num_inputs);
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
                            let overlap =
                                (2 * hamming_dist) as f64 / circuit_one.num_wires as f64 - 1.0;
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
        "analyze-replacements" => {
            let repl_sample_path = args.next().expect("Missing trace.json path");
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
        "display" => {
            let circuit_path = args.next().unwrap();
            let circuit = Circuit::load_from_json(&circuit_path);
            println!("{}", circuit_path);
            println!("{}", circuit.to_string_vertical());
        }
        _ => {
            eprintln!("Unknown command: {}", cmd);
        }
    }
}
