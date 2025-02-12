use std::time::Duration;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ReplacementStats<const ITER: usize> {
    pub times: Vec<Duration>,
}

impl<const ITER: usize> ReplacementStats<ITER> {
    pub fn new() -> Self {
        Self {
            times: Vec::with_capacity(ITER),
        }
    }

    pub fn add_entry(&mut self, time: Duration) {
        self.times.push(time);
    }

    pub fn at_capacity(&self) -> bool {
        self.times.len() == ITER
    }

    pub fn clear(&mut self) {
        self.times.clear();
    }
}
