use bincode;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;

use crate::circuit::Gate;

#[derive(Serialize, Deserialize)]
pub struct CircuitTable {
    max_gates_supported: usize,
    max_wires_supported: usize,
    cf_choice: Vec<u8>,
    table: HashMap<Vec<usize>, Vec<Gate>>,
}

impl CircuitTable {
    pub fn new(max_gates_supported: usize, max_wires_supported: usize, cf_choice: Vec<u8>) -> Self {
        let table = build_compression_table(max_gates_supported, max_wires_supported, &cf_choice);

        Self {
            max_gates_supported,
            max_wires_supported,
            cf_choice,
            table,
        }
    }

    pub fn from_file(path: &str) -> Self {
        let file = File::open(path).expect("Unable to open file");
        bincode::deserialize_from(file).expect("Deserialization failed")
    }

    pub fn save_to_file(&self, path: &str) {
        let mut file = File::create(path).expect("Unable to create file");
        let encoded: Vec<u8> = bincode::serialize(self).expect("Serialization failed");
        file.write_all(&encoded).expect("Unable to write data");
    }
}

pub fn build_compression_table(
    max_gates_supported: usize,
    max_wires_supported: usize,
    cf_choice: &Vec<u8>,
) -> HashMap<Vec<usize>, Vec<Gate>> {
    let tt_size = 1 << max_wires_supported;

    let mut table = HashMap::new();

    table.insert((0..tt_size).collect(), vec![]);

    for current_idx in 1..=max_gates_supported {
        
    }

    table
}
