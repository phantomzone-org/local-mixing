use std::usize;

use crate::{
    circuit::Gate,
    local_mixing::consts::{N_IN, N_PROJ_WIRES},
};
use rand::{seq::SliceRandom, RngCore, SeedableRng};
use rand_chacha::ChaCha8Rng;

use super::{permutations::{LFSR16,walksman_permutation_8}, strategy::ControlFnChoice};

///////////////////////////////////////////////////////////////////////////////////////////////////
// helper functions for lfsr 128
///////////////////////////////////////////////////////////////////////////////////////////////////
fn get_tap_128(state: u128, n: u128) -> u128 {
    (state & (1 << n)) >> n
}

fn get_128_from_u8_array(data: [u8; 16]) -> u128 {
    let mut value: u128 = 0;
    for i in 0..16 {
        value += (data[i] << (i * 8)) as u128;
    }
    value
}

fn get_u8_array_from_u128(value: u128) -> [u8; 16] {
    let mut data = [0; 16];
    for i in 0..16 {
        data[i] = (value >> (i * 8)) as u8;
    }
    data
}


///////////////////////////////////////////////////////////////////////////////////////////////////
// LFSR that satisfies RngCore
///////////////////////////////////////////////////////////////////////////////////////////////////
#[derive(Debug, Clone, Copy)]
pub struct LFSR128 {
    state: u128,
}

impl LFSR128 {
    pub fn new(seed: u128) -> Self {
        Self { state: seed }
    }
}

impl Iterator for LFSR128 {
    type Item = u128;

    fn next(&mut self) -> Option<Self::Item> {
        let new = get_tap_128(self.state, 127)
            ^ get_tap_128(self.state, 126)
            ^ get_tap_128(self.state, 125)
            ^ get_tap_128(self.state, 120);
        let old = self.state;
        self.state = (self.state << 1) | (new);

        Some(old)
    }
}

impl LFSR128 {
    pub fn sample_excluding<const N_PROJ_WIRES: usize>(
        &mut self,
        exclusion_values: &Vec<usize>,
    ) -> usize {
        loop {
            let new = self.next().unwrap() as usize % N_PROJ_WIRES;
            if !exclusion_values.contains(&new) {
                return new;
            }
        }
    }
}

impl RngCore for LFSR128 {
    fn next_u32(&mut self) -> u32 {
        self.next().unwrap_or(0) as u32
    }

    fn next_u64(&mut self) -> u64 {
        self.next().unwrap_or(0) as u64
    }

    fn fill_bytes(&mut self, dest: &mut [u8]) {
        // calculating the number of u8 vectors that needs to be created.
        let max_len = (dest.len() + 15) / 16;
        let mut store: Vec<u8> = vec![];
        // populating the vector
        for _i in 0..max_len {
            store.append(&mut get_u8_array_from_u128(self.next().unwrap_or(0)).into())
        }
        // storing the values
        for index in 0..dest.len() {
            dest[index] = store[index];
        }
    }
}

impl SeedableRng for LFSR128 {
    /// Define the type of the seed. Here we use a single-element array of `u8` for simplicity.
    type Seed = [u8; 16];
    fn from_seed(seed: Self::Seed) -> Self {
        Self {
            state: get_128_from_u8_array(seed),
        }
    }
}

///////////////////////////////////////////////////////////////////////////////////////////////////
// Struct for handling wire permutations
///////////////////////////////////////////////////////////////////////////////////////////////////
#[derive(Debug, Clone, Copy)]
pub struct WireEntries {
    position: u8,
    present: bool,
}

impl Default for WireEntries {
    fn default() -> Self {
        Self {
            position: 0,
            present: false,
        }
    }
}

///////////////////////////////////////////////////////////////////////////////////////////////////
// Main shuffling object
///////////////////////////////////////////////////////////////////////////////////////////////////
#[derive(Clone)]
pub struct LFSRShuffle {
    control_matrix: [[WireEntries; N_IN]; 2],
    targets: [WireEntries; N_IN],
    permutor: LFSR16,
    rng: ChaCha8Rng,
    lfsr: LFSR128,
}

impl LFSRShuffle {
    ///////////////////////////////////////////////////////////////////////////////////////////////////
    // Setup logic
    ///////////////////////////////////////////////////////////////////////////////////////////////////
    pub fn new<R: RngCore>(active_wires: [[bool; N_PROJ_WIRES]; 2], rng: &mut R) -> Self {
        // extracting the actual target and control wires
        let mut true_active_target_wires = active_wires[0]
            .into_iter()
            .enumerate()
            .filter(|(_, x)| *x)
            .map(|(i, _)| i)
            .collect::<Vec<usize>>();

        let mut true_active_control_wires = active_wires[1]
            .into_iter()
            .enumerate()
            .filter(|(_, x)| *x)
            .map(|(i, _)| i)
            .collect::<Vec<usize>>();

        assert!(
            true_active_control_wires.len() <= (2 * N_IN),
            "The control wires have to be lesser than and equal 2 * N_IN"
        );
        assert!(
            true_active_target_wires.len() <= (N_IN),
            "The control wires have to be lesser and equal than N_IN"
        );

        // shuffling the targets and controls
        true_active_control_wires.shuffle(rng);
        true_active_target_wires.shuffle(rng);

        let mut control_matrix: [[WireEntries; 4]; 2] = [[WireEntries::default(); N_IN]; 2];
        let mut targets: [WireEntries; N_IN] = [WireEntries::default(); N_IN];

        // populating the matrix and the target rows
        for i in 0..N_IN {
            control_matrix[0][i] = match true_active_control_wires.get(i) {
                Some(v) => WireEntries {
                    position: (*v as u8),
                    present: true,
                },
                None => WireEntries {
                    position: 0,
                    present: false,
                },
            };
            control_matrix[1][i] = match true_active_control_wires.get(N_IN + i) {
                Some(v) => WireEntries {
                    position: (*v as u8),
                    present: true,
                },
                None => WireEntries {
                    position: 0,
                    present: false,
                },
            };
            targets[i] = match true_active_target_wires.get(i) {
                Some(v) => WireEntries {
                    position: (*v as u8),
                    present: true,
                },
                None => WireEntries {
                    position: 0,
                    present: false,
                },
            };
        }

        let lfsr_seed: u128 = ((rng.next_u64() as u128) << 64) + (rng.next_u64() as u128);

        // let mut control_matrix: [[WireEntries; N_IN];2];
        Self {
            control_matrix,
            targets,
            permutor: LFSR16::new(rng.next_u32() as u16),
            rng: ChaCha8Rng::from_rng(rng),
            lfsr: LFSR128::new(lfsr_seed),
        }
    }

    ///////////////////////////////////////////////////////////////////////////////////////////////////
    // Helper functions
    ///////////////////////////////////////////////////////////////////////////////////////////////////

    fn collision_check(&self) -> bool {
        for i in 0..N_IN {
            if self.control_matrix[0][i].present & self.targets[i].present {
                if self.targets[i].position == self.control_matrix[0][i].position {
                    return true;
                }
            }

            if self.control_matrix[1][i].present & self.targets[i].present {
                if self.targets[i].position == self.control_matrix[1][i].position {
                    return true;
                }
            }
        }
        false
    }

    ///////////////////////////////////////////////////////////////////////////////////////////////////
    // Placing random wires where there are not active wires.
    ///////////////////////////////////////////////////////////////////////////////////////////////////
    fn place_gate_wires(&mut self, index: usize) -> Gate {
        let mut gate = Gate {
            wires: [0; 3],
            control_func: 0,
        };
        let mut exclusion_store = vec![];

        // placing the targets in position 0
        gate.wires[0] = match self.targets[index].present {
            true => {
                exclusion_store.push(self.targets[index].position as usize);
                self.targets[index].position as u32
            }
            false => {
                let val = self.lfsr.sample_excluding::<N_PROJ_WIRES>(&exclusion_store);
                exclusion_store.push(val as usize);
                val as u32
            }
        };

        // placing the control in position 1 & 2
        gate.wires[1] = match self.control_matrix[0][index].present {
            true => {
                exclusion_store.push(self.control_matrix[0][index].position as usize);
                self.control_matrix[0][index].position as u32
            }
            false => {
                let val = self.lfsr.sample_excluding::<N_PROJ_WIRES>(&exclusion_store);
                exclusion_store.push(val as usize);
                val as u32
            }
        };
        gate.wires[2] = match self.control_matrix[1][index].present {
            true => self.control_matrix[1][index].position as u32,
            false => {
                let val = self.lfsr.sample_excluding::<N_PROJ_WIRES>(&exclusion_store);
                val as u32
            }
        };
        gate
    }
}
///////////////////////////////////////////////////////////////////////////////////////////////////
// Here all the implementations of the shuffle algorithm
///////////////////////////////////////////////////////////////////////////////////////////////////
impl LFSRShuffle {
    ///////////////////////////////////////////////////////////////////////////////////////////////////
    // shuffle with only rotations
    ///////////////////////////////////////////////////////////////////////////////////////////////////
    fn shuffle(&mut self) {
        // getting the rotation indexes
        let rotation_count_1 = (self.lfsr.next().unwrap() as usize) % N_IN;
        let rotation_count_2 = (self.lfsr.next().unwrap() as usize) % N_IN;
        let target_count = (self.lfsr.next().unwrap() as usize) % N_IN;

        for i in 0..N_IN {
            if i < rotation_count_1 {
                self.control_matrix[0].rotate_left(1);
            }
            if i < rotation_count_2 {
                self.control_matrix[1].rotate_left(1);
            }
            if i < target_count {
                self.targets.rotate_left(1);
            }

            let column_swap_index: usize = (self.lfsr.next().unwrap() % (N_IN as u128)) as usize;
            // if get_tap_128(self.lfsr.state, column_swap_index as u128) == 1 {
            let temp = self.control_matrix[0][column_swap_index];
            self.control_matrix[0][column_swap_index] = self.control_matrix[1][column_swap_index];
            self.control_matrix[1][column_swap_index] = temp;
            // }
        }
    }

    ///////////////////////////////////////////////////////////////////////////////////////////////////
    // shuffle with riffle and swaps
    ///////////////////////////////////////////////////////////////////////////////////////////////////
    fn shuffle_riffle(&mut self) {
        // getting the rotation indexes
        let column_swap_count = (self.lfsr.next().unwrap() as usize) % N_IN;

        // targets permuted
        self.targets = self.permutor.riffle_array(&self.targets);
        // control permuted
        self.control_matrix[0] = self.permutor.riffle_array(&self.control_matrix[0]);
        self.control_matrix[1] = self.permutor.riffle_array(&self.control_matrix[1]);

        for _ in 0..column_swap_count {
            let column_swap_index: usize = (self.lfsr.next().unwrap() % (N_IN as u128)) as usize;
            let temp = self.control_matrix[0][column_swap_index];
            self.control_matrix[0][column_swap_index] = self.control_matrix[1][column_swap_index];
            self.control_matrix[1][column_swap_index] = temp;
        }
    }

    ///////////////////////////////////////////////////////////////////////////////////////////////////
    // shuffle with permutation networks and riffle
    ///////////////////////////////////////////////////////////////////////////////////////////////////
    fn shuffle_permute(&mut self) {
        // getting the rotation indexes
        let state = self.next_u32() as usize;

        // targets permuted
        self.targets = self.permutor.permute_4(&self.targets);

        // control permuted
        let mut columns = [WireEntries::default(); 2 * N_IN];

        for i in 0..(2 * N_IN) {
            if i < N_IN {
                columns[i] = self.control_matrix[0][i];
            } else {
                // println!("The index is {} -> {}/ {}", i, N_IN - i, i - N_IN);
                columns[i] = self.control_matrix[1][i - N_IN];
            }
        }

        let mut control = vec![];
        for index in 0..17 {
            control.push(((state >> index) & 1) == 1);
        }
        columns = self.permutor.riffle_array(&columns);
        columns = walksman_permutation_8(&columns, control);

        for i in 0..(2 * N_IN) {
            if i < N_IN {
                self.control_matrix[0][i] = columns[i];
            } else {
                self.control_matrix[1][i - N_IN] = columns[i];
            }
        }
    }

    ///////////////////////////////////////////////////////////////////////////////////////////////////
    // Base rust array shuffle implementation
    ///////////////////////////////////////////////////////////////////////////////////////////////////
    fn rng_shuffle(&mut self) {
        // targets permuted
        self.targets.shuffle(&mut self.rng);

        let mut columns = [WireEntries::default(); 2 * N_IN];

        for i in 0..(2 * N_IN) {
            if i < N_IN {
                columns[i] = self.control_matrix[0][i];
            } else {
                // println!("The index is {} -> {}/ {}", i, N_IN - i, i - N_IN);
                columns[i] = self.control_matrix[1][i - N_IN];
            }
        }
        columns.shuffle(&mut self.rng);

        for i in 0..(2 * N_IN) {
            if i < N_IN {
                self.control_matrix[0][i] = columns[i];
            } else {
                self.control_matrix[1][i - N_IN] = columns[i];
            }
        }
    }

    ///////////////////////////////////////////////////////////////////////////////////////////////////
    // shuffle with riffle, swaps and chacha rng
    ///////////////////////////////////////////////////////////////////////////////////////////////////
    fn shuffle_rng(&mut self) {
        // getting the rotation indexes
        let rotation_count_1 = (self.rng.next_u32() as usize) % N_IN;
        let rotation_count_2 = (self.rng.next_u32() as usize) % N_IN;
        let target_count = (self.rng.next_u32() as usize) % N_IN;

        for i in 0..N_IN {
            if i < rotation_count_1 {
                self.control_matrix[0] = self.permutor.riffle_array(&self.control_matrix[0]);
            }
            if i < rotation_count_2 {
                self.control_matrix[1] = self.permutor.riffle_array(&self.control_matrix[1]);
            }

            if i < target_count {
                self.targets = self.permutor.riffle_array(&self.targets);
            }

            let rand_val = self.rng.next_u32() as usize;
            let column_swap_index: usize = rand_val % N_IN;
            let temp = self.control_matrix[0][column_swap_index];
            self.control_matrix[0][column_swap_index] = self.control_matrix[1][column_swap_index];
            self.control_matrix[1][column_swap_index] = temp;
        }
    }
}

pub trait GateProvider {
    fn get_gates(&mut self, gates: &mut [Gate; N_IN], cf_choice: ControlFnChoice);
}

impl GateProvider for LFSRShuffle {
    ///////////////////////////////////////////////////////////////////////////////////////////////////
    // The core logic for permuting the active control and target wire
    // and then placing random wires in the remaing position to build
    // a replacement circuit that can be used.
    ///////////////////////////////////////////////////////////////////////////////////////////////////
    fn get_gates(&mut self, gates: &mut [Gate; N_IN], cf_choice: ControlFnChoice) {
        loop {
            // self.shuffle();
            // self.shuffle_riffle();
            // self.shuffle_rng();
            self.shuffle_permute();
            // self.rng_shuffle();
            if self.collision_check() == false {
                break;
            }
        }

        for i in 0..N_IN {
            // assigning the wires
            gates[i] = self.place_gate_wires(i);
            // assigning gates
            gates[i].control_func = cf_choice.random_cf(&mut self.lfsr);
        }
    }
}

///////////////////////////////////////////////////////////////////////////////////////////////////
// Satisfying RngCore for the whole object as well
///////////////////////////////////////////////////////////////////////////////////////////////////
impl RngCore for LFSRShuffle {
    fn next_u32(&mut self) -> u32 {
        self.lfsr.next_u32()
    }

    fn next_u64(&mut self) -> u64 {
        self.lfsr.next_u64()
    }

    fn fill_bytes(&mut self, dest: &mut [u8]) {
        self.lfsr.fill_bytes(dest);
    }
}

