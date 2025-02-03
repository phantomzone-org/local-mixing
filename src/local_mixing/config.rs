use std::{error::Error, fs::File, io::BufReader};

use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct LocalMixingConfig {
    pub num_wires: u32,
    pub num_inflationary_steps: usize,
    pub num_kneading_steps: usize,
    pub num_replacement_attempts: usize,
    pub num_inflationary_to_fail: usize,
    pub num_kneading_to_fail: usize,
    pub epoch_size: usize,
}

impl LocalMixingConfig {
    pub fn load_from_file(path: &str) -> Result<Self, Box<dyn Error>> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let config = serde_json::from_reader(reader)?;
        Ok(config)
    }
}
