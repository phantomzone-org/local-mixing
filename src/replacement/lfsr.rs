use std::cmp::max;

use rand::{seq::SliceRandom, RngCore, SeedableRng};
use crate::{circuit::Gate, local_mixing::consts::{N_IN, N_PROJ_WIRES}};

use super::strategy::ControlFnChoice;

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
    pub fn sample_excluding(&mut self, exclusion_values: &Vec<usize>) -> u128 {
        loop {
            let new = self.next().unwrap_or(0);
            if !exclusion_values.contains(&(new as usize)) {
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

#[derive(Debug,Clone, Copy)]
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

#[derive(Clone, Copy)]
pub struct LFSRShuffle {
    control_matrix: [[WireEntries; N_IN]; 2],
    targets: [WireEntries; N_IN],
    lfsr: LFSR128,
}

impl LFSRShuffle {
    pub fn new<R: RngCore>(active_wires: [[bool; N_PROJ_WIRES]; 2], rng: &mut R) -> Self {
        // extracting the actual target and control wires
        let mut true_active_target_wires = active_wires[0]
            .into_iter()
            .enumerate()
            .filter(|(_, x)| *x)
            .map(|(i, _)| i)
            .collect::<Vec<usize>>();

        let mut  true_active_control_wires = active_wires[1]
            .into_iter()
            .enumerate()
            .filter(|(_, x)| *x)
            .map(|(i, _)| i)
            .collect::<Vec<usize>>();

        assert!(true_active_control_wires.len() <= (2*N_IN), "The control wires have to be lesser than and equal 2 * N_IN");
        assert!(true_active_target_wires.len() <= (N_IN), "The control wires have to be lesser and equal than N_IN");

        // shuffling the targets and controls
        true_active_control_wires.shuffle(rng);
        true_active_target_wires.shuffle(rng);

        let mut control_matrix: [[WireEntries; 4]; 2] = [[WireEntries::default();N_IN];2];
        let mut targets: [WireEntries; N_IN] = [WireEntries::default();N_IN];
        
        // populating the matrix and the target rows
        for i in 0..N_IN {
            control_matrix[0][i] = match true_active_control_wires.get(i) {
                Some(v) => WireEntries{position: (*v as u8), present:true},
                None => WireEntries{position:0, present:false}
            };
            control_matrix[1][i] = match true_active_control_wires.get(N_IN + i) {
                Some(v) => WireEntries{position: (*v as u8), present:true},
                None => WireEntries{position:0, present:false}
            };
            targets[i] = match true_active_target_wires.get(i) {
                Some(v) => WireEntries{position: (*v as u8), present:true},
                None => WireEntries{position:0, present:false}
            };
        }

        let lfsr_seed: u128 = ((rng.next_u64() as u128) << 64) + (rng.next_u64() as u128);

        // let mut control_matrix: [[WireEntries; N_IN];2];
        Self {
            control_matrix,
            targets,
            lfsr: LFSR128::new(lfsr_seed),
        }
    }

    pub fn set_seed<R: Send + Sync + RngCore + SeedableRng>(&mut self, rng: &mut R) {
        self.lfsr.state = ((rng.next_u64() as u128) << 64) + (rng.next_u64() as u128);
    }

    fn shuffle(&mut self) {
        // getting the rotation indexes
        let rotation_index_1 = (self.lfsr.next().unwrap_or(0) as usize) % N_IN;
        let rotation_index_2 = (self.lfsr.next().unwrap_or(0) as usize) % N_IN;
        let target_index = (self.lfsr.next().unwrap_or(0) as usize) % N_IN;
        let max_rotations = max(rotation_index_1, max(rotation_index_2, target_index));

        for i in 0..max_rotations {
            if i < rotation_index_1{
                self.control_matrix[0].rotate_left(get_tap_128(self.lfsr.state, rotation_index_1 as u128) as usize)
            }
            if i < rotation_index_2{
                self.control_matrix[1].rotate_left(get_tap_128(self.lfsr.state, rotation_index_2 as u128) as usize)
            }

            if i < target_index {
                self.targets.rotate_left(get_tap_128(self.lfsr.state, target_index as u128) as usize);
            }
            let column_swap_index: usize = (self.lfsr.state % (N_IN as u128) ) as usize; 
            if get_tap_128(self.lfsr.state, 0) == 1 {
                let temp = self.control_matrix[0][column_swap_index];
                self.control_matrix[0][column_swap_index] = self.control_matrix[1][column_swap_index];
                self.control_matrix[1][column_swap_index] = temp;
            }
        }
        
    }
}

pub trait GateProvider {
    fn get_gates(&mut self, gates: &mut [Gate; N_IN], cf_choice: ControlFnChoice);
}

impl GateProvider for LFSRShuffle {
    fn get_gates(&mut self, gates: &mut [Gate; N_IN], cf_choice: ControlFnChoice) {

        self.shuffle();

        for i in 0..N_IN {
            // assigning the wires
            // will assign a wire if there are no active wires
            let mut exclusion_store = vec![];
            if self.control_matrix[0][i].present {
                exclusion_store.push(self.control_matrix[0][i].position as usize)
            }
            if self.control_matrix[1][i].present {
                exclusion_store.push(self.control_matrix[0][i].position as usize)
            }

            if self.targets[i].present {
                exclusion_store.push(self.control_matrix[0][i].position as usize)
            }

            gates[i].wires[0] = match self.control_matrix[0][i].present {
                true => self.control_matrix[0][i].position as u32,
                false => (self.lfsr.sample_excluding(&exclusion_store) as u32) % (N_PROJ_WIRES as u32),
            };

            gates[i].wires[1] = match self.control_matrix[1][i].present {
                true => self.control_matrix[1][i].position as u32,
                false => (self.lfsr.sample_excluding(&exclusion_store) as u32) % (N_PROJ_WIRES as u32),
            };

            gates[i].wires[2] = match self.targets[i].present {
                true => self.targets[i].position as u32,
                false => (self.lfsr.sample_excluding(&exclusion_store) as u32) % (N_PROJ_WIRES as u32),
            };
            // assigning gates
            gates[i].control_func = cf_choice.random_cf(&mut self.lfsr);
        }
    }
}

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


mod tests {
    use rand_chacha::ChaCha8Rng;

    use super::*;

    #[test]
    fn test_shuffle() {
       let mut active_wires = [[false;N_PROJ_WIRES];2];
       let mut rng  = ChaCha8Rng::from_os_rng();
       for i in 0..6{
        active_wires[i % 2][i] = true;
       }
       println!("The active wires are {:?}",active_wires);
       let mut shuff = LFSRShuffle::new(active_wires, &mut rng);

       println!("The initial state matrix :{:?}, targets: {:?}", shuff.control_matrix, shuff.targets);
       shuff.shuffle();
       println!("The shuffled state matrix :{:?}, targets: {:?}", shuff.control_matrix, shuff.targets);
       shuff.shuffle();
       println!("The shuffled state matrix :{:?}, targets: {:?}", shuff.control_matrix, shuff.targets);
       shuff.shuffle();
       println!("The shuffled state matrix :{:?}, targets: {:?}", shuff.control_matrix, shuff.targets);

    }
}