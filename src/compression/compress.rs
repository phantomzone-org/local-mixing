use crate::{
    circuit::{
        circuit::{check_ckt_equiv_inout_map, correct_controls, evaluate},
        Circuit, Gate,
    },
    compression::ct::CompressionTable,
    local_mixing::search::{find_convex_gate_ids3, permute_circuit, random_convex},
};
use rand::Rng;
use std::path::Path;

pub fn compress(circuit_path: impl AsRef<Path>) {
    let mut circuit = Circuit::load_from_json(&circuit_path);
    dbg!(circuit.gates.len());
    let save_dir = circuit_path
        .as_ref()
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap();
    dbg!(&save_dir);
    let mut rng = rand::rng();
    correct_controls(&mut circuit.gates);

    let inout: Vec<(Vec<bool>, Vec<bool>)> = (0..1000)
        .map(|_| {
            let input = (0..circuit.num_wires)
                .map(|_| rng.random_bool(0.5))
                .collect();
            let output = evaluate(&circuit.gates, &input);
            (input, output)
        })
        .collect();

    println!("inout computed");

    let ct = CompressionTable::from_file("bin/table-all.db");

    println!("ct loaded");

    // simplify_identity_pairs(&mut circuit.gates);
    // simplify_cxity_one_pairs(&mut circuit.gates);
    assert!(check_ckt_equiv_inout_map(&inout, &circuit.gates));

    circuit.save_as_json(format!(
        "{}/latest.{}.json",
        save_dir.display(),
        circuit.gates.len()
    ));
    dbg!(circuit.gates.len());

    loop {
        for _ in 0..100000 {
            ct_compress_active_wires_single_step(
                circuit.num_wires,
                &mut circuit.gates,
                &ct,
                &mut rng,
            );
        }
        // simplify_identity_pairs(&mut circuit.gates);
        // simplify_cxity_one_pairs(&mut circuit.gates);
        assert!(check_ckt_equiv_inout_map(&inout, &circuit.gates));
        circuit.save_as_json(format!(
            "{}/latest.{}.json",
            save_dir.display(),
            circuit.gates.len()
        ));
        dbg!(circuit.gates.len());
    }
}

#[allow(dead_code)]
fn compress_block(circuit_gates: &mut Vec<Gate>, ct: &CompressionTable) {
    for set_size in (2..=10).rev() {
        let mut i = 0;
        while i < circuit_gates.len() - set_size {
            if let Some(replacement) =
                ct.compress_with_optimal_relabel(&circuit_gates[i..i + set_size])
            {
                let repl_len = replacement.len();
                circuit_gates.splice(i..i + set_size, replacement);
                println!(
                    "Removed {} gates by compress_block. # gates: {}",
                    set_size - repl_len,
                    circuit_gates.len()
                );
            } else {
                i += 1;
            }
        }
    }
}

#[allow(dead_code)]
fn ct_compress_single_step<R: Rng>(
    circuit_num_wires: usize,
    circuit_gates: &mut Vec<Gate>,
    ct: &CompressionTable,
    rng: &mut R,
) {
    let set_size = rng.random_range(2..=10);
    let (selected_gate_idx, _) = find_convex_gate_ids3(
        set_size,
        ct.max_wires_supported,
        circuit_num_wires,
        circuit_gates,
        rng,
    );
    let selected_gates: Vec<_> = selected_gate_idx
        .iter()
        .map(|&i| circuit_gates[i])
        .collect();

    if let Some(replacement) = ct.compress(&selected_gates) {
        let repl_len = replacement.len();
        let start = permute_circuit(circuit_num_wires, circuit_gates, &selected_gate_idx);
        circuit_gates.splice(start..start + set_size, replacement);
        println!(
            "Removed {} gates by ct_compress_single_step. # gates: {}",
            set_size - repl_len,
            circuit_gates.len()
        );
    }
}

fn ct_compress_active_wires_single_step<R: Rng>(
    circuit_num_wires: usize,
    circuit_gates: &mut Vec<Gate>,
    ct: &CompressionTable,
    rng: &mut R,
) {
    let set_size = rng.random_range(2..=10);
    let max_wires = rng.random_range(set_size..set_size + 2);
    // let wc = rng.random_bool(0.5);
    let wc = true;
    let (selected_gate_idx, _) = match wc {
        true => find_convex_gate_ids3(
            set_size,
            ct.max_wires_supported,
            circuit_num_wires,
            &circuit_gates,
            rng,
        ),
        false => random_convex(set_size, max_wires, circuit_num_wires, &circuit_gates, rng),
    };
    let selected_gates: Vec<_> = selected_gate_idx
        .iter()
        .map(|&i| circuit_gates[i])
        .collect();

    if let Some(replacement) = ct.compress_with_optimal_relabel(&selected_gates) {
        let repl_len = replacement.len();
        let start = permute_circuit(circuit_num_wires, circuit_gates, &selected_gate_idx);
        circuit_gates.splice(start..start + set_size, replacement);
        println!(
            "Removed {} gates by ct_compress_single_step. # gates: {}",
            set_size - repl_len,
            circuit_gates.len()
        );
    }
}

fn simplify_identity_pairs(circuit_gates: &mut Vec<Gate>) {
    let old_len = circuit_gates.len();
    let mut prev_len = circuit_gates.len() + 1;
    while circuit_gates.len() < prev_len {
        prev_len = circuit_gates.len();
        simplify_identity_pairs_single_pass(circuit_gates);
    }
    println!(
        "Removed {} gates by simplify_identity_pairs",
        old_len - circuit_gates.len()
    );
}

fn simplify_identity_pairs_single_pass(circuit_gates: &mut Vec<Gate>) {
    let mut i = 0;
    while i < circuit_gates.len() {
        let mut j = i + 1;
        while j < circuit_gates.len() {
            if circuit_gates[i].equal_to(&circuit_gates[j]) {
                break;
            }
            j += 1;
        }
        if j != circuit_gates.len()
            && (i + 1..j).all(|id| !circuit_gates[i].collides_with(&circuit_gates[id]))
        {
            circuit_gates.remove(j);
            circuit_gates.remove(i);
        } else {
            i += 1;
        }
    }
}

fn simplify_cxity_one_pairs(circuit_gates: &mut Vec<Gate>) {
    let old_len = circuit_gates.len();
    let mut prev_len = circuit_gates.len() + 1;
    while circuit_gates.len() < prev_len {
        prev_len = circuit_gates.len();
        simplify_cxity_one_pairs_single_pass(circuit_gates);
        correct_non_influential_controls(circuit_gates);
    }
    println!(
        "Removed {} gates by simplify_cxity_one_pairs",
        old_len - circuit_gates.len()
    );
}

fn simplify_cxity_one_pairs_single_pass(circuit_gates: &mut Vec<Gate>) {
    let mut i = 0;
    while i < circuit_gates.len() {
        let mut j = i + 1;
        while j < circuit_gates.len() {
            if circuit_gates[i].wires == circuit_gates[j].wires {
                break;
            }
            j += 1;
        }
        if j != circuit_gates.len()
            && (i + 1..j).all(|id| !circuit_gates[i].collides_with(&circuit_gates[id]))
        {
            let cf_j = circuit_gates.remove(j).control_func;
            circuit_gates[i].control_func = combine_cfs(circuit_gates[i].control_func, cf_j);
        } else {
            i += 1;
        }
    }
}

fn combine_cfs(cf1: u8, cf2: u8) -> u8 {
    const TABLE: [[u8; 16]; 16] = [
        [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15], // false
        [1, 0, 3, 2, 5, 4, 7, 6, 9, 8, 11, 10, 13, 12, 15, 14], // a & b
        [2, 3, 0, 1, 6, 7, 4, 5, 10, 11, 8, 9, 14, 15, 12, 13], // a & (!b)
        [3, 2, 1, 0, 7, 6, 5, 4, 11, 10, 9, 8, 15, 14, 13, 12], // a
        [4, 5, 6, 7, 0, 1, 2, 3, 12, 13, 14, 15, 8, 9, 10, 11], // (!a) & b
        [5, 4, 7, 6, 1, 0, 3, 2, 13, 12, 15, 14, 9, 8, 11, 10], // b
        [6, 7, 4, 5, 2, 3, 0, 1, 14, 15, 12, 13, 10, 11, 8, 9], // a ^ b
        [7, 6, 5, 4, 3, 2, 1, 0, 15, 14, 13, 12, 11, 10, 9, 8], // a | b
        [8, 9, 10, 11, 12, 13, 14, 15, 0, 1, 2, 3, 4, 5, 6, 7], // !(a | b)
        [9, 8, 11, 10, 13, 12, 15, 14, 1, 0, 3, 2, 5, 4, 7, 6], // (a & b) | ((!a) & (!b))
        [10, 11, 8, 9, 14, 15, 12, 13, 2, 3, 0, 1, 6, 7, 4, 5], // !b
        [11, 10, 9, 8, 15, 14, 13, 12, 3, 2, 1, 0, 7, 6, 5, 4], // (!b) | a
        [12, 13, 14, 15, 8, 9, 10, 11, 4, 5, 6, 7, 0, 1, 2, 3], // !a
        [13, 12, 15, 14, 9, 8, 11, 10, 5, 4, 7, 6, 1, 0, 3, 2], // (!a) | b
        [14, 15, 12, 13, 10, 11, 8, 9, 6, 7, 4, 5, 2, 3, 0, 1], // !(a & b)
        [15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0], // true
    ];

    TABLE[cf1 as usize][cf2 as usize]
}

fn correct_non_influential_controls(circuit_gates: &mut Vec<Gate>) {
    for g in circuit_gates.iter_mut() {
        if g.control_func == 0 || g.control_func == 15 {
            // false, true
            g.wires[1] = 0;
            g.wires[2] = 0;
        } else if g.control_func == 3 || g.control_func == 12 {
            // a, !a
            g.wires[2] = 0;
        } else if g.control_func == 5 || g.control_func == 10 {
            // b, !b
            g.wires[1] = 0;
        }
    }
}
