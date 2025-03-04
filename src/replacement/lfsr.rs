
use std::usize;

use rand::{seq::SliceRandom, RngCore, SeedableRng};
use rand_chacha::ChaCha8Rng;
use crate::{circuit::Gate, local_mixing::consts::{N_IN, N_PROJ_WIRES}};

use super::strategy::ControlFnChoice;

fn get_tap_128(state: u128, n: u128) -> u128 {
    (state & (1 << n)) >> n
}

fn get_tap(state: usize, n: usize) -> usize {
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

#[inline]
fn get_4<T: Default + Copy + Sized>(a: &[T], start_index: usize) -> [T;4] {
    let mut out = [T::default();4];
    for i in 0..4 {
        out[i] = a[start_index + i];
    }
    out
}

#[inline]
fn fill_4<T: Default + Copy + Sized>(a: &mut [T], b: &[T], start_index: usize) {
    for i in 0..4 {
        a[start_index + i] = b[i];
    }
}

fn walksman_permutation_4<T: Default + Copy + Sized> (a: &[T;4], control: &[bool]) -> [T; 4] {
    let mut out = a.clone();

    // first layer swaps
    if control[0] {
        out.swap(0, 1);
    }

    if control[1] {
        out.swap(2, 3);
    }
    
    // second layer swaps
    if control[2] {
        out.swap(0, 2);
    }

    if control[3] {
        out.swap(1, 3);
    }

    // second layer swaps
    if control[4] {
        out.swap(0, 1);
    }
    
    out
}


fn walksman_permutation_8<T: Default + Copy + Sized> (a: &[T;8], control: Vec<bool>) -> [T; 8] {
    let mut out = a.clone();

    // first layer swaps
    if control[0] {
        out.swap(0, 1);
    }

    if control[1] {
        out.swap(2, 3);
    }
    if control[2] {
        out.swap(4, 5);
    }

    if control[3] {
        out.swap(6, 7);
    }

    // intermerate layers
    let first_half: [T; 4] = [out[0], out[2], out[4], out[6]];
    let interim_1 = walksman_permutation_4(&first_half,&control[4..]);
    // let interim_1 = permutation_net_4(&first_half,control[4..].to_vec());

    let second_half: [T; 4] = [out[1], out[3], out[5], out[7]];
    let interim_2 = walksman_permutation_4(&second_half,&control[9..]);
    // let interim_2 = permutation_net_4(&second_half,control[9..].to_vec());

    for i in 0..8 {
        if i % 2 == 0 {
            out[i] = interim_1[i/2];
        } else {
            out[i] = interim_2[i/2];
        }
    }

    // last layer swaps
    if control[14] {
        out.swap(0, 1);
    }

    if control[15] {
        out.swap(2, 3);
    }
    if control[16] {
        out.swap(4, 5);
    }
    
    out
}

fn permutation_net_4<T: Default + Copy> (a: &[T;4], control: Vec<bool>) -> [T; 4] {
    let mut out = a.clone();

    // first layer swaps
    if control[0] {
        out.swap(0, 1);
    }

    if control[1] {
        out.swap(2, 3);
    }
    
    // second layer swaps
    if control[2] {
        out.swap(1, 2);
    }

    // second layer swaps
    if control[3] {
        out.swap(0, 1);
    }

    if control[4] {
        out.swap(2, 3);
    }
    
    out
}




// pub fn walksman_permutation <const N_IN: usize, T: Default + Copy + Clone > (a: &[T;N_IN], control: Vec<bool>) -> [T; N_IN] {
//     let  out = [T::default(); N_IN];

//     if N_IN == 4 {
//         let input: [T; 4]  = &a[..4].into();
//         let permuted: [T; 4] = walksman_permutation_4(&input, control);
//         out[..4].copy_from_slice(&permuted);
//     }

//     out
// }

/// Performs a riffle shuffle on a fixed-size array.
///
/// This function simulates a single riffle shuffle.
/// The behavior depends on whether the number of elements (`N_IN`) is even or odd,
/// and on the value of the `even` flag:
///
/// - **Even-length deck (`N_IN % 2 == 0`):**
///   - If `even` is `true`, the deck is split exactly in half. The left half is placed
///     into the even-index positions (0, 2, 4, …) and the right half into the odd-index positions.
///   - If `even` is `false`, the roles are reversed: the right half goes to even positions
///     and the left half to odd positions.
///
/// - **Odd-length deck (`N_IN % 2 == 1`):**
///   - The deck cannot be split exactly in half, so one half will contain one extra element.
///   - If `even` is `true`, the extra (middle) element is assigned to the left half.
///     In this case, the left slice includes indices `0..=half_point` (i.e. the middle element is included),
///     and the right slice starts at `half_point + 1`.
///   - If `even` is `false`, the extra element is assigned to the right half.
///     Here, the left slice is `0..half_point` and the right slice is `half_point..` (with the middle element in the right half).
///
/// The function then interleaves the two slices by:
/// - For each index `i` in the output:
///   - If `i` is even, it takes the element from the first (either left or right, as determined by `even`)
///     slice at index `i/2`.
///   - If `i` is odd, it takes the element from the other slice at index `i/2`.
///
/// Finally, the shuffled array is returned without modifying the original input.
///
/// # Examples
///
/// ```
/// // For a 5-element deck:
/// // When even == true, left = [a[0], a[1], a[2]] and right = [a[3], a[4]]
/// // Output order: [left[0], right[0], left[1], right[1], left[2]]
/// let deck = [1, 2, 3, 4, 5];
/// let shuffled = riffle_shuffle(deck, true);
/// println!("{:?}", shuffled); // e.g., [1, 4, 2, 5, 3]
///
/// // For a 4-element deck with even == false:
/// // left = [a[0], a[1]], right = [a[2], a[3]]
/// // Output order: [right[0], left[0], right[1], left[1]]
/// let deck = [10, 20, 30, 40];
/// let shuffled = riffle_shuffle(deck, false);
/// println!("{:?}", shuffled); // e.g., [30, 10, 40, 20]
fn riffle_shuffle<const N_IN: usize, T: Default + Copy> (a: &[T;N_IN], even: bool) -> [T; N_IN] {
    let mut out = [T::default(); N_IN];
    let half_point: usize = N_IN / 2;
    if N_IN % 2 == 1  {
        if even {
            let left  = &a[0..=half_point];
            let right = &a[(half_point + 1)..];
            for i in 0..N_IN {
                if i %2 == 0 {
                  out[i] = left[i / 2];
                } else {
                  out[i] = right[i / 2];
                }
            }
        } else {
            let left = &a[0..half_point];
            let right = &a[half_point..];
            for i in 0..N_IN {
                if i %2 == 0 {
                  out[i] = right[i / 2];
                } else {
                  out[i] = left[i / 2];
                }
            }
        }
    } else {
        let left = &a[0..half_point];
        let right = &a[half_point..];
        if even {
            for i in 0..N_IN {
              if i %2 == 0 {
                out[i] = left[i / 2];
              } else {
                out[i] = right[i / 2];
              }
            }
        } else {
            for i in 0..N_IN {
                if i %2 == 0 {
                  out[i] = right[i / 2];
                } else {
                  out[i] = left[i / 2];
                }
            }
        }
    }
    out 
}
#[derive(Clone)]
pub struct LFSR16 {
    state: u16
}

impl LFSR16 {
    pub fn new(seed: u16) -> Self {
        Self { state: seed }
    }

    fn sample(&mut self) -> u16 {
        let old  = self.state;
        let new_bit = ((self.state >> 15) & 1) ^ ((self.state >> 13) & 1) ^ ((self.state >> 12) & 1) ^ ((self.state >> 10) & 1) ;
        self.state = (self.state << 1) | new_bit;
        old 
    }

    // riffle_array 
    // uses the state of the first bit riffle the array 
    // and 5th bit to either rotate or not
    pub fn riffle_array<const N_IN: usize, T: Default + Copy> (&mut self,a: &[T;N_IN]) -> [T; N_IN] {
        let state = self.sample();

        let mut out = riffle_shuffle(a, (state & 1) == 1);
        out.rotate_left(((state >> 4) & 1) as usize);
        out
    }

    pub fn permute_4<T: Default + Copy> (&mut self,a: &[T;4]) -> [T; 4] {
        let mut control = vec![];
        for _i in 0..5 {
            let state = self.sample();
            control.push(((state ) & 1) == 1);
        }

        walksman_permutation_4(a, &control)
    }
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
    pub fn sample_excluding<const N_IN:usize>(&mut self, exclusion_values: &Vec<usize>) -> u128 {
        loop {
            let new = self.next().unwrap_or(0) % (N_IN as u128);
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

#[derive(Clone)]
pub struct LFSRShuffle{
    control_matrix: [[WireEntries; N_IN]; 2],
    targets: [WireEntries; N_IN],
    permutor: LFSR16,
    rng: ChaCha8Rng,
    lfsr: LFSR128,
}

impl LFSRShuffle {
    pub fn new<R: RngCore> (active_wires: [[bool; N_PROJ_WIRES]; 2], rng: &mut R) -> Self {
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
            permutor: LFSR16::new(rng.next_u32() as u16),
            rng: ChaCha8Rng::from_rng(rng),
            lfsr: LFSR128::new(lfsr_seed),
        }
    }

    fn collision_check(&self) -> bool {
        for i in 0..N_IN {
            if self.control_matrix[0][i].present & self.targets[i].present {
                if self.targets[i].position == self.control_matrix[0][i].position {
                    return true;
                }
            }

            if self.control_matrix[0][i].present & self.targets[i].present {
                if self.targets[i].position == self.control_matrix[0][i].position {
                    return true;
                }
            }
        }
        false
    }

    fn shuffle(&mut self) {
        // getting the rotation indexes
        let rotation_count_1 = (self.lfsr.next().unwrap() as usize) % N_IN;
        let rotation_count_2 = (self.lfsr.next().unwrap() as usize) % N_IN;
        let target_count = (self.lfsr.next().unwrap() as usize) % N_IN;
        // let max_rotations = max(max(rotation_count_1, column_swap_count), max(rotation_count_2, target_count));

        for i in 0..N_IN {
            if i < rotation_count_1{
                self.control_matrix[0].rotate_left(1);
            }
            if i < rotation_count_2{
                self.control_matrix[1].rotate_left(1);
            }
            if i < target_count {
                self.targets.rotate_left(1);
            }

            let column_swap_index: usize = (self.lfsr.next().unwrap() % (N_IN as u128) ) as usize; 
            // if get_tap_128(self.lfsr.state, column_swap_index as u128) == 1 {
            let temp = self.control_matrix[0][column_swap_index];
            self.control_matrix[0][column_swap_index] = self.control_matrix[1][column_swap_index];
            self.control_matrix[1][column_swap_index] = temp;
            // }
        }
        
    }

    fn shuffle_riffle(&mut self) {
        // getting the rotation indexes
        let column_swap_count = (self.lfsr.next().unwrap() as usize) % N_IN;
        
        // targets permuted
        self.targets = self.permutor.riffle_array(&self.targets);
        // control permuted
        self.control_matrix[0] = self.permutor.riffle_array(&self.control_matrix[0]);
        self.control_matrix[1] = self.permutor.riffle_array(&self.control_matrix[1]);

        for _ in 0..column_swap_count{
            let column_swap_index: usize = (self.lfsr.next().unwrap() % (N_IN as u128) ) as usize; 
            // if get_tap_128(self.lfsr.state, column_swap_index as u128) == 1 {
            let temp = self.control_matrix[0][column_swap_index];
            self.control_matrix[0][column_swap_index] = self.control_matrix[1][column_swap_index];
            self.control_matrix[1][column_swap_index] = temp;
            // }
        }
        
    }

    fn shuffle_permute(&mut self) {
        // getting the rotation indexes
        let state = self.rng.next_u32() as usize;
        
        // targets permuted
        self.targets = self.permutor.permute_4(&self.targets);
        // control permuted
        self.control_matrix[0] = self.permutor.permute_4(&self.control_matrix[0]);
        self.control_matrix[1] = self.permutor.permute_4(&self.control_matrix[1]);


        let mut columns = [WireEntries::default(); 2*N_IN];

        for i in 0..(2*N_IN) {
            if i < N_IN {
                columns[i] =  self.control_matrix[0][i];
            } else {
                // println!("The index is {} -> {}/ {}", i, N_IN - i, i - N_IN);
                columns[i] =  self.control_matrix[1][i - N_IN];
            }
        }

        let mut control = vec![];
        for index in 0..17 {
            control.push((( state >> index) & 1) == 1);
        }
        columns = walksman_permutation_8(&columns, control);

        for i in 0..(2*N_IN) {
            if i < N_IN {
                self.control_matrix[0][i] = columns[i];
            } else {
                self.control_matrix[1][i - N_IN] = columns[i];
            }
        }
        
    }

    fn shuffle_rng(&mut self) {
        // getting the rotation indexes
        let rotation_count_1 = (self.rng.next_u32() as usize) % N_IN;
        let rotation_count_2 = (self.rng.next_u32() as usize) % N_IN;
        let target_count = (self.rng.next_u32() as usize) % N_IN;

        for i in 0..N_IN {
            if i < rotation_count_1{
                self.control_matrix[0] = self.permutor.riffle_array(&self.control_matrix[0]);
            }
            if i < rotation_count_2{
                self.control_matrix[1] = self.permutor.riffle_array(&self.control_matrix[1]);
            }

            if i < target_count {
                self.targets = self.permutor.riffle_array(&self.targets);
            }

            let rand_val = self.rng.next_u32() as usize;
            let column_swap_index: usize = rand_val % N_IN ;
            let temp = self.control_matrix[0][column_swap_index];
            self.control_matrix[0][column_swap_index] = self.control_matrix[1][column_swap_index];
            self.control_matrix[1][column_swap_index] = temp;
        }
        
    }
}

pub trait GateProvider {
    fn get_gates(&mut self, gates: &mut [Gate; N_IN], cf_choice: ControlFnChoice);
}

impl GateProvider for LFSRShuffle{
    fn get_gates(&mut self, gates: &mut [Gate; N_IN], cf_choice: ControlFnChoice) {
        loop {
            // self.shuffle();
            // self.shuffle_riffle();
            // self.shuffle_rng();
            self.shuffle_permute();
            if self.collision_check() == false {
                break;
            }
        }

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
                false => (self.lfsr.sample_excluding::<N_IN>(&exclusion_store) as u32) % (N_PROJ_WIRES as u32),
            };

            gates[i].wires[1] = match self.control_matrix[1][i].present {
                true => self.control_matrix[1][i].position as u32,
                false => (self.lfsr.sample_excluding::<N_IN>(&exclusion_store) as u32) % (N_PROJ_WIRES as u32),
            };

            gates[i].wires[2] = match self.targets[i].present {
                true => self.targets[i].position as u32,
                false => (self.lfsr.sample_excluding::<N_IN>(&exclusion_store) as u32) % (N_PROJ_WIRES as u32),
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
    use std::collections::{HashSet, HashMap};

    use rand::Rng;
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

    #[test]
    fn test_riffle(){
        let a:[usize; 4] = [1, 2, 3, 4];
        let b = riffle_shuffle(&a, false);
        println!("The actual values are {:?} -> {:?}", a, b);
        assert!(b == [3, 1, 4, 2], "There should be 24 values");

        let b = riffle_shuffle(&a, true);
        println!("The actual values are {:?} -> {:?}", a, b);
        assert!(b == [1, 3, 2, 4], "There should be 24 values");

        let mut rng = ChaCha8Rng::from_os_rng();

        let mut lfsr = LFSR16::new(rng.next_u32() as u16);

        let mut set = HashSet::new();
        let mut b = a.clone();
        set.insert(b.clone());
        for _i in 0..200 {
            // println!("The values before {:?}", b);
            b = lfsr.riffle_array(&b);
            set.insert(b.clone());
            // println!("The values after {:?} state = {}", b, even);
        }

        println!("The set is values are {:?}", set);
        assert!(set.len() == 24, "There should be 24 values");

        let mut map = HashMap::new();
        let mut b = a.clone();
        map.insert(b.clone(), 0);
        for _i in 0..240000 {
            // println!("The values before {:?}", b);
            b = lfsr.riffle_array(&b);
            if map.contains_key(&b) {
                map.insert(b.clone(), map.get(&b).unwrap() + 1);
            } else {
                map.insert(b.clone(), 0);
            }
            
            // println!("The values after {:?} state = {}", b, even);
        }

        println!("The map values are {:?}", map);
        assert!(map.len() == 24, "There should be 24 values");
    }

    #[test]
    fn test_permute_4(){
        let a:[usize; 4] = [1, 2, 3, 4];
        let b = walksman_permutation_4(&a, &vec![true, false, false, false, false]);
        println!("The actual values are {:?} -> {:?}", a, b);
        assert!(b == [2, 1, 3, 4]);

        let mut rng = ChaCha8Rng::from_os_rng();

        let mut lfsr = LFSR16::new(rng.next_u32() as u16);

        let mut set = HashSet::new();
        let mut b = a.clone();
        set.insert(b.clone());
        for _i in 0..200 {
            // println!("The values before {:?}", b);
            b = lfsr.permute_4(&b);
            set.insert(b.clone());
            // println!("The values after {:?} state = {}", b, even);
        }

        println!("The set is values are {:?}", set);
        assert!(set.len() == 24, "There should be 24 values");

        let mut map = HashMap::new();
        let mut b = a.clone();
        map.insert(b.clone(), 0);
        for _i in 0..240000 {
            // println!("The values before {:?}", b);
            b = lfsr.permute_4(&b);
            if map.contains_key(&b) {
                map.insert(b.clone(), map.get(&b).unwrap() + 1);
            } else {
                map.insert(b.clone(), 0);
            }
            
            // println!("The values after {:?} state = {}", b, even);
        }

        println!("The lfsr map values are {:?}", map);
        assert!(map.len() == 24, "There should be 24 values");

        let mut map = HashMap::new();
        let mut b = a.clone();
        map.insert(b.clone(), 0);
        for _i in 0..240000 {
            // println!("The values before {:?}", b);
            b = walksman_permutation_4(&b, &vec![(rng.next_u32() %2 == 1), (rng.next_u32() %2 == 1), (rng.next_u32() %2 == 1), (rng.next_u32() %2 == 1), (rng.next_u32() %2 == 1)]);
            if map.contains_key(&b) {
                map.insert(b.clone(), map.get(&b).unwrap() + 1);
            } else {
                map.insert(b.clone(), 0);
            }
            
            // println!("The values after {:?} state = {}", b, even);
        }

        println!("The walksman map values are {:?}", map);
        assert!(map.len() == 24, "There should be 24 values");

        let mut map = HashMap::new();
        let mut b = a.clone();
        map.insert(b.clone(), 0);
        for _i in 0..240000 {
            // println!("The values before {:?}", b);
            b = permutation_net_4(&b, vec![(rng.next_u32() %2 == 1), (rng.next_u32() %2 == 1), (rng.next_u32() %2 == 1), (rng.next_u32() %2 == 1), (rng.next_u32() %2 == 1),]);
            if map.contains_key(&b) {
                map.insert(b.clone(), map.get(&b).unwrap() + 1);
            } else {
                map.insert(b.clone(), 0);
            }
            
            // println!("The values after {:?} state = {}", b, even);
        }

        println!("The perm net map values are {:?}", map);
        assert!(map.len() == 24, "There should be 24 values");

    }

    #[test]
    fn test_permute_8(){
        let a:[usize; 8] = [1, 2, 3, 4, 5, 6, 7, 8];
        let b = walksman_permutation_8(&a, vec![true, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false]);
        println!("The actual values are {:?} -> {:?}", a, b);
        assert!(b == [2, 1, 3, 4, 5, 6, 7, 8]);

        // let mut rng = ChaCha8Rng::from_os_rng();

        // let mut lfsr = LFSR16::new(rng.next_u32() as u16);

        let mut map = HashMap::new();
        let mut b = a.clone();
        map.insert(b.clone(), 0);
        for i in 0..(1<<18) {
            // println!("The values before {:?}", b);
            let mut control = vec![];
            for index in 0..17 {
                control.push((( i >> index) & 1) == 1);
            }
            b = walksman_permutation_8(&b, control);
            if map.contains_key(&b) {
                map.insert(b.clone(), map.get(&b).unwrap() + 1);
            } else {
                map.insert(b.clone(), 0);
            }
            
            // println!("The values after {:?} state = {}", b, even);
        }

        println!("The walksman 8 map values are {:?}", map.len());
        assert!(map.len() == 40320, "There should be 40320 values");

    }

    #[test]
    fn test_riffle_5(){
        let a:[usize; 5] = [1, 2, 3, 4, 5];

        let mut rng = ChaCha8Rng::from_os_rng();

        let mut lfsr = LFSR16::new(rng.next_u32() as u16);

        let mut set = HashSet::new();
        let mut b = a.clone();
        set.insert(b.clone());
        for _i in 0..20000 {
            // b.rotate_right(1);
            b = lfsr.riffle_array(&b);
            set.insert(b.clone());
        }

        println!("The set for 5 is {:?}", set);
        assert!(set.len() == 120, "There should be 120 values");
    }
}