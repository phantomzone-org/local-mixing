use rand_chacha::ChaCha8Rng;
use rand::{seq::SliceRandom, RngCore, SeedableRng};
///////////////////////////////////////////////////////////////////////////////////////////////////
// functions for permutation networks
///////////////////////////////////////////////////////////////////////////////////////////////////
fn walksman_permutation_4<T: Default + Copy + Sized>(a: &[T; 4], control: &[bool]) -> [T; 4] {
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

///////////////////////////////////////////////////////////////////////////////////////////////////
// Walksman permutation for size 8
///////////////////////////////////////////////////////////////////////////////////////////////////
pub fn walksman_permutation_8<T: Default + Copy + Sized>(a: &[T; 8], control: Vec<bool>) -> [T; 8] {
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
    let interim_1 = walksman_permutation_4(&first_half, &control[4..]);

    let second_half: [T; 4] = [out[1], out[3], out[5], out[7]];
    let interim_2 = walksman_permutation_4(&second_half, &control[9..]);

    for i in 0..8 {
        if i % 2 == 0 {
            out[i] = interim_1[i / 2];
        } else {
            out[i] = interim_2[i / 2];
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

///////////////////////////////////////////////////////////////////////////////////////////////////
// functions for permutation networks of size 5
///////////////////////////////////////////////////////////////////////////////////////////////////
fn walksman_permutation_5<T: Default + Copy + Sized>(a: &[T; 5], control: &[bool]) -> [T; 5] {
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

    if control[4] {
        out.swap(3, 4);
    }

    if control[5] {
        out.swap(1, 3);
    }

    // third layer swaps
    if control[6] {
        out.swap(0, 1);
    }

    if control[7] {
        out.swap(2, 3);
    }

    out
}

///////////////////////////////////////////////////////////////////////////////////////////////////
// functions for permutation networks of size 10
///////////////////////////////////////////////////////////////////////////////////////////////////
fn walksman_permutation_10<T: Default + Copy + Sized>(a: &[T; 10], control: &[bool]) -> [T; 10] {
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

    if control[4] {
        out.swap(8, 9);
    }

    // intermerate layers
    let mut first_half: [T; 5] = [T::default(); 5];
    let mut second_half: [T; 5] = [T::default(); 5];
    for i in 0..10 {
        if i % 2 == 0 {
            first_half[i / 2] = out[i];
        } else {
            second_half[i / 2] = out[i];
        }
    }
    // let first_half: [T; 4] = [out[0], out[2], out[4], out[6]];
    let interim_1 = walksman_permutation_5(&first_half, &control[5..]);

    // let second_half: [T; 4] = [out[1], out[3], out[5], out[7]];
    let interim_2 = walksman_permutation_5(&second_half, &control[13..]);

    for i in 0..10 {
        if i % 2 == 0 {
            out[i] = interim_1[i / 2];
        } else {
            out[i] = interim_2[i / 2];
        }
    }

    // last layer swaps
    if control[21] {
        out.swap(0, 1);
    }

    if control[22] {
        out.swap(2, 3);
    }
    if control[23] {
        out.swap(4, 5);
    }

    if control[24] {
        out.swap(6, 7);
    }

    out
}


///////////////////////////////////////////////////////////////////////////////////////////////////
// functions for riffling and array
///////////////////////////////////////////////////////////////////////////////////////////////////

/// Performs a riffle shuffle on a fixed-size array.
///
/// This function simulates a single riffle shuffle.
/// The behavior depends on whether the number of elements (`N_IN`) is even or odd,
/// and on the value of the `even` flag:
///
/// The function splits the array in half and then interleaves the two slices by:
/// - For each index `i` in the output:
///   - If `i` is even, it takes the element from the first (either left or right, as determined by `even`)
///     slice at index `i/2`.
///   - If `i` is odd, it takes the element from the other slice at index `i/2`.
fn riffle_shuffle<const N_IN: usize, T: Default + Copy>(a: &[T; N_IN], even: bool) -> [T; N_IN] {
    let mut out = [T::default(); N_IN];
    let half_point: usize = N_IN / 2;
    if N_IN % 2 == 1 {
        if even {
            let left = &a[0..=half_point];
            let right = &a[(half_point + 1)..];
            for i in 0..N_IN {
                if i % 2 == 0 {
                    out[i] = left[i / 2];
                } else {
                    out[i] = right[i / 2];
                }
            }
        } else {
            let left = &a[0..half_point];
            let right = &a[half_point..];
            for i in 0..N_IN {
                if i % 2 == 0 {
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
                if i % 2 == 0 {
                    out[i] = left[i / 2];
                } else {
                    out[i] = right[i / 2];
                }
            }
        } else {
            for i in 0..N_IN {
                if i % 2 == 0 {
                    out[i] = right[i / 2];
                } else {
                    out[i] = left[i / 2];
                }
            }
        }
    }
    out
}

///////////////////////////////////////////////////////////////////////////////////////////////////
// LFSR's
///////////////////////////////////////////////////////////////////////////////////////////////////
#[derive(Clone)]
pub struct LFSR16 {
    state: u16,
}

impl LFSR16 {
    pub fn new(seed: u16) -> Self {
        Self { state: seed }
    }

    fn sample(&mut self) -> u16 {
        let old = self.state;
        let new_bit = ((self.state >> 15) & 1)
            ^ ((self.state >> 13) & 1)
            ^ ((self.state >> 12) & 1)
            ^ ((self.state >> 10) & 1);
        self.state = (self.state << 1) | new_bit;
        old
    }

    // riffle_array
    // uses the state of the first bit riffle the array
    // and 5th bit to either rotate or not
    pub fn riffle_array<const N_IN: usize, T: Default + Copy>(
        &mut self,
        a: &[T; N_IN],
    ) -> [T; N_IN] {
        let state = self.sample();

        let mut out = riffle_shuffle(a, (state & 1) == 1);
        out.rotate_left(((state >> 4) & 1) as usize);
        out
    }

    pub fn permute_4<T: Default + Copy>(&mut self, a: &[T; 4]) -> [T; 4] {
        let mut control = vec![];
        for _i in 0..5 {
            let state = self.sample();
            control.push(((state) & 1) == 1);
        }

        walksman_permutation_4(a, &control)
    }
}



mod tests {
    use std::collections::{HashMap, HashSet};

    use rand::Rng;
    use rand_chacha::ChaCha8Rng;

    use std::fs::File;
    use std::io::Write;
    use serde_json;


    use super::*;

    #[test]
    fn test_permute_10() {
        const DUMP: bool = true;
        ///////////////////////////////////////////////////////////////////////////////////////////////////
        // base test
        ///////////////////////////////////////////////////////////////////////////////////////////////////
        let a: [usize; 10] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let mut b = walksman_permutation_10(
            &a,
            &vec![
                true, false, false, false, false, false, false, false,
                false, false, false, false, false, false, false, false,
                false, false, false, false, false, false, false, false,
                false
            ],
        );
        println!("The actual values are {:?} -> {:?}", a, b);
        assert!(b == [2, 1, 3, 4, 5, 6, 7, 8, 9, 10]);

        let mut rng = ChaCha8Rng::from_os_rng();

        let mut lfsr = LFSR16::new(rng.next_u32() as u16);

        ///////////////////////////////////////////////////////////////////////////////////////////////////
        // Testing the permutation and shuffle for size 8
        ///////////////////////////////////////////////////////////////////////////////////////////////////
        
        #[derive(Clone, Debug)]
        struct Perm {
            frequency: usize,
            controls: Vec<Vec<bool>>,
        }
        if !DUMP {
        let mut map: HashMap<[usize;10], Perm> = HashMap::new();
        // map.insert(b.clone(), 0);
        for i in 0..(1 << 26) {
            let mut control = vec![];
            for index in 0..27 {
                control.push(((i >> index) & 1) == 1);
            }
            b = a.clone();
            // b = lfsr.riffle_array(&b);
            b = walksman_permutation_10(&b, &control);
            if map.contains_key(&b) {
                let mut new_perm: Perm = map.get(&b).unwrap().clone();
                new_perm.frequency += 1;
                new_perm.controls.push(control.clone());
                map.insert(b.clone(), new_perm);
            } else {
                let mut store_of_controls = vec![];
                store_of_controls.push(control.clone());
                map.insert(b.clone(), Perm{
                    frequency: 1,
                    controls: store_of_controls,
                });
            }
        }

        println!(
            "The walksman 10 with same input map values are {:?}",
            map
            // .iter()
            // .filter(|(_, v)| v.frequency >= 8)
            // .map(|(k,v)| (k.clone(), v.clone()))
            // .map(|(k, v)| (k.clone(), v.controls.iter().map(|x|{
            //     let number_of_true = x.iter().filter(|&&b| b).count();
            //     (number_of_true, (x.len() - number_of_true))
            // }).collect::<Vec<(usize, usize)>>()))
            // .collect::<Vec<([usize;8],Vec<(usize, usize)>)>>()
            // .collect::<HashMap<[usize;5], Perm>>()
            .len()
        );
        assert!(map.len() == 3628800, "There should be 3628800 values");
        }

        ///////////////////////////////////////////////////////////////////////////////////////////////////
        // Testing the permutation and shuffle for size 8
        ///////////////////////////////////////////////////////////////////////////////////////////////////

        // let mut map: HashMap<[usize;10], Perm> = HashMap::new();
        // // map.insert(b.clone(), 0);
        // b = a.clone();
        // for _i in 0..(1 << 31) {
        //     let mut control = vec![];
        //     let seed = rng.next_u64();
        //     for index in 0..27 {
        //         control.push((( seed as usize >> index) & 1) == 1);
        //     }
        //     b = lfsr.riffle_array(&b);
        //     b = walksman_permutation_10(&b, &control);
        //     // b.rotate_right(rng)
        //     if map.contains_key(&b) {
        //         let mut new_perm: Perm = map.get(&b).unwrap().clone();
        //         new_perm.frequency += 1;
        //         new_perm.controls.push(control.clone());
        //         map.insert(b.clone(), new_perm);
        //     } else {
        //         let mut store_of_controls = vec![];
        //         store_of_controls.push(control.clone());
        //         map.insert(b.clone(), Perm{
        //             frequency: 1,
        //             controls: store_of_controls,
        //         });
        //     }
        // }

        // // println!(
        // //     "The walksman 5 with same input map values are {:?}",
        // //     map
        // //     .iter()
        // //     .filter(|(_, v)| v.frequency > 20)
        // //     .map(|(k,v)| (k.clone(), v.clone()))
        // //     // .map(|(k, v)| (k.clone(), v.controls.iter().map(|x|{
        // //     //     let number_of_true = x.iter().filter(|&&b| b).count();
        // //     //     (number_of_true, (x.len() - number_of_true))
        // //     // }).collect::<Vec<(usize, usize)>>()))
        // //     // .collect::<Vec<([usize;8],Vec<(usize, usize)>)>>()
        // //     .collect::<HashMap<[usize;5], Perm>>()
        // //     .len()
        // // );
        // assert!(map.len() == 3628800, "There should be 3628800 values");


        // ///////////////////////////////////////////////////////////////////////////////////////////////////
        // // testing the frequncy of occurance of each value in the different positions
        // // for riffle with permutation
        // ///////////////////////////////////////////////////////////////////////////////////////////////////
        if DUMP {
        
        println!("The walksman 10 with riffle testing uniformity.");
        let mut occurance = HashMap::new();
        let mut b: [usize; 10] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        for i in 1..=10 {
            occurance.insert(i, [0 as usize; 10]);
        }
        println!("The test base {:?}.", occurance);
        for i in 0..(1 << 30) {
            // print()
            let mut control = vec![];
            let seed = rng.next_u64();
            for index in 0..27 {
                control.push(((seed as usize >> index) & 1) == 1);
            }
            b = lfsr.riffle_array(&b);
            b = walksman_permutation_10(&b, &control);
            for index in 1..=10 {
                let position = b.iter().position(|&x| x == index).unwrap();
                let mut values = *occurance.get(&index).unwrap();
                values[position] += 1;
                occurance.insert(index, values);
            }
            if i == (1<<29) {
                println!("Reached 2 ** 29");
            }
        }

        // println!("The walksman 10 occurance of values are {:?}", occurance);
        
        // Convert to JSON string
        let json_string = serde_json::to_string_pretty(&occurance).unwrap();
        // Write to a file
        let mut file = File::create("riffle-permute.json").unwrap();
        file.write_all(json_string.as_bytes()).unwrap();

        let reduced_map: HashMap<usize, Vec<usize>> = occurance.iter().map(|(k, v)| {
            (*k, v.iter().map(|&x| x/ 100_000).collect::<Vec<usize>>())
        }).collect();

        // Convert to JSON string
        let json_string = serde_json::to_string_pretty(&reduced_map).unwrap();
        // Write to a file
        let mut file = File::create("riffle-permute-reduced.json").unwrap();
        file.write_all(json_string.as_bytes()).unwrap();
        }
        ///////////////////////////////////////////////////////////////////////////////////////////////////
        // testing the frequncy of occurance of each value in the different positions
        // for in built shuffle
        ///////////////////////////////////////////////////////////////////////////////////////////////////
        if DUMP {
        println!("The shuffle in built for size 10 testing uniformity.");
        let mut occurance = HashMap::new();
        let mut b: [usize; 10] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        for i in 1..=10 {
            occurance.insert(i, [0 as usize; 10]);
        }

        for _ in 0..(1 << 25) {
            b.shuffle(&mut rng);
            for index in 1..=10{
                let position = b.iter().position(|&x| x == index).unwrap();
                let mut values = *occurance.get(&index).unwrap();
                values[position] += 1;
                occurance.insert(index, values);
            }
        }

        // println!("The walksman 8 occurance of values are {:?}", occurance);
        
         // Convert to JSON string
         let json_string = serde_json::to_string_pretty(&occurance).unwrap();
         // Write to a file
         let mut file = File::create("rust-shuffle.json").unwrap();
         file.write_all(json_string.as_bytes()).unwrap();
        }
    }


    #[test]
    fn test_permute_5() {
        const DUMP: bool = true;
        ///////////////////////////////////////////////////////////////////////////////////////////////////
        // base test
        ///////////////////////////////////////////////////////////////////////////////////////////////////
        let a: [usize; 5] = [1, 2, 3, 4, 5];
        let mut b = walksman_permutation_5(
            &a,
            &vec![
                true, false, false, false, false, false, false, false
            ],
        );
        println!("The actual values are {:?} -> {:?}", a, b);
        assert!(b == [2, 1, 3, 4, 5]);

        let mut rng = ChaCha8Rng::from_os_rng();

        let mut lfsr = LFSR16::new(rng.next_u32() as u16);

        ///////////////////////////////////////////////////////////////////////////////////////////////////
        // Testing the permutation and shuffle for size 8
        ///////////////////////////////////////////////////////////////////////////////////////////////////
        
        #[derive(Clone, Debug)]
        struct Perm {
            frequency: usize,
            controls: Vec<Vec<bool>>,
        }

        let mut map: HashMap<[usize;5], Perm> = HashMap::new();
        // map.insert(b.clone(), 0);
        for i in 0..(1 << 8) {
            let mut control = vec![];
            for index in 0..8 {
                control.push(((i >> index) & 1) == 1);
            }
            b = a.clone();
            // b = lfsr.riffle_array(&b);
            b = walksman_permutation_5(&b, &control);
            if map.contains_key(&b) {
                let mut new_perm: Perm = map.get(&b).unwrap().clone();
                new_perm.frequency += 1;
                new_perm.controls.push(control.clone());
                map.insert(b.clone(), new_perm);
            } else {
                let mut store_of_controls = vec![];
                store_of_controls.push(control.clone());
                map.insert(b.clone(), Perm{
                    frequency: 1,
                    controls: store_of_controls,
                });
            }
        }

        println!(
            "The walksman 5 with same input map values are {:?}",
            map
            .iter()
            // .filter(|(_, v)| v.frequency >= 8)
            // .map(|(k,v)| (k.clone(), v.clone()))
            // .map(|(k, v)| (k.clone(), v.controls.iter().map(|x|{
            //     let number_of_true = x.iter().filter(|&&b| b).count();
            //     (number_of_true, (x.len() - number_of_true))
            // }).collect::<Vec<(usize, usize)>>()))
            // .collect::<Vec<([usize;8],Vec<(usize, usize)>)>>()
            // .collect::<HashMap<[usize;5], Perm>>()
            .len()
        );
        assert!(map.len() == 120, "There should be 120 values");

        ///////////////////////////////////////////////////////////////////////////////////////////////////
        // Testing the permutation and shuffle for size 8
        ///////////////////////////////////////////////////////////////////////////////////////////////////

        let mut map: HashMap<[usize;5], Perm> = HashMap::new();
        // map.insert(b.clone(), 0);
        b = a.clone();
        for _i in 0..(1 << 11) {
            let mut control = vec![];
            for index in 0..8 {
                control.push(((rng.next_u64() as usize >> index) & 1) == 1);
            }
            b = lfsr.riffle_array(&b);
            b = walksman_permutation_5(&b, &control);
            // b.rotate_right(rng)
            if map.contains_key(&b) {
                let mut new_perm: Perm = map.get(&b).unwrap().clone();
                new_perm.frequency += 1;
                new_perm.controls.push(control.clone());
                map.insert(b.clone(), new_perm);
            } else {
                let mut store_of_controls = vec![];
                store_of_controls.push(control.clone());
                map.insert(b.clone(), Perm{
                    frequency: 1,
                    controls: store_of_controls,
                });
            }
        }

        println!(
            "The walksman 5 with same input map values are {:?}",
            map
            .iter()
            .filter(|(_, v)| v.frequency > 20)
            .map(|(k,v)| (k.clone(), v.clone()))
            // .map(|(k, v)| (k.clone(), v.controls.iter().map(|x|{
            //     let number_of_true = x.iter().filter(|&&b| b).count();
            //     (number_of_true, (x.len() - number_of_true))
            // }).collect::<Vec<(usize, usize)>>()))
            // .collect::<Vec<([usize;8],Vec<(usize, usize)>)>>()
            .collect::<HashMap<[usize;5], Perm>>()
            .len()
        );
        assert!(map.len() == 120, "There should be 120 values");


        // ///////////////////////////////////////////////////////////////////////////////////////////////////
        // // testing the frequncy of occurance of each value in the different positions
        // // for riffle with permutation
        // ///////////////////////////////////////////////////////////////////////////////////////////////////

        let mut occurance = HashMap::new();
        let mut b: [usize; 5] = [1, 2, 3, 4, 5];
        for i in 1..=5 {
            occurance.insert(i, [0 as usize; 5]);
        }

        for _i in 0..(1 << 25) {
            let mut control = vec![];
            let seed = rng.next_u64();
            for index in 0..17 {
                control.push(((seed as usize >> index) & 1) == 1);
            }
            b = lfsr.riffle_array(&b);
            b = walksman_permutation_5(&b, &control);
            for index in 1..=5 {
                let position = b.iter().position(|&x| x == index).unwrap();
                let mut values = *occurance.get(&index).unwrap();
                values[position] += 1;
                occurance.insert(index, values);
            }
        }

        println!("The walksman 8 occurance of values are {:?}", occurance);
        if DUMP {
        // Convert to JSON string
        let json_string = serde_json::to_string_pretty(&occurance).unwrap();
        // Write to a file
        let mut file = File::create("riffle-permute.json").unwrap();
        file.write_all(json_string.as_bytes()).unwrap();

        let reduced_map: HashMap<usize, Vec<usize>> = occurance.iter().map(|(k, v)| {
            (*k, v.iter().map(|&x| x/ 100_000).collect::<Vec<usize>>())
        }).collect();

        // Convert to JSON string
        let json_string = serde_json::to_string_pretty(&reduced_map).unwrap();
        // Write to a file
        let mut file = File::create("riffle-permute-reduced.json").unwrap();
        file.write_all(json_string.as_bytes()).unwrap();
        }
        ///////////////////////////////////////////////////////////////////////////////////////////////////
        // testing the frequncy of occurance of each value in the different positions
        // for in built shuffle
        ///////////////////////////////////////////////////////////////////////////////////////////////////

        let mut occurance = HashMap::new();
        let mut b: [usize; 5] = [1, 2, 3, 4, 5];
        for i in 1..=5 {
            occurance.insert(i, [0 as usize; 5]);
        }

        for _ in 0..(1 << 25) {
            b.shuffle(&mut rng);
            for index in 1..=5{
                let position = b.iter().position(|&x| x == index).unwrap();
                let mut values = *occurance.get(&index).unwrap();
                values[position] += 1;
                occurance.insert(index, values);
            }
        }

        println!("The walksman 8 occurance of values are {:?}", occurance);
        if DUMP {
         // Convert to JSON string
         let json_string = serde_json::to_string_pretty(&occurance).unwrap();
         // Write to a file
         let mut file = File::create("rust-shuffle.json").unwrap();
         file.write_all(json_string.as_bytes()).unwrap();
        }
    }


    #[test]
    fn test_riffle() {
        ///////////////////////////////////////////////////////////////////////////////////////////////////
        // base case
        ///////////////////////////////////////////////////////////////////////////////////////////////////
        let a: [usize; 4] = [1, 2, 3, 4];
        let b = riffle_shuffle(&a, false);
        println!("The actual values are {:?} -> {:?}", a, b);
        assert!(b == [3, 1, 4, 2], "There should be 24 values");

        let b = riffle_shuffle(&a, true);
        println!("The actual values are {:?} -> {:?}", a, b);
        assert!(b == [1, 3, 2, 4], "There should be 24 values");

        let mut rng = ChaCha8Rng::from_os_rng();

        let mut lfsr = LFSR16::new(rng.next_u32() as u16);

        ///////////////////////////////////////////////////////////////////////////////////////////////////
        // testing if all values are reached
        ///////////////////////////////////////////////////////////////////////////////////////////////////

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

        ///////////////////////////////////////////////////////////////////////////////////////////////////
        // testing the distribution of values
        ///////////////////////////////////////////////////////////////////////////////////////////////////

        let mut map = HashMap::new();
        let mut b = a.clone();
        map.insert(b.clone(), 0);
        for _i in 0..240000 {
            b = lfsr.riffle_array(&b);
            if map.contains_key(&b) {
                map.insert(b.clone(), map.get(&b).unwrap() + 1);
            } else {
                map.insert(b.clone(), 0);
            }
        }

        println!("The map values are {:?}", map);
        assert!(map.len() == 24, "There should be 24 values");
    }

    #[test]
    fn test_permute_4() {
        ///////////////////////////////////////////////////////////////////////////////////////////////////
        // base test
        ///////////////////////////////////////////////////////////////////////////////////////////////////
        let a: [usize; 4] = [1, 2, 3, 4];
        let b = walksman_permutation_4(&a, &vec![true, false, false, false, false]);
        println!("The actual values are {:?} -> {:?}", a, b);
        assert!(b == [2, 1, 3, 4]);

        let mut rng = ChaCha8Rng::from_os_rng();

        let mut lfsr = LFSR16::new(rng.next_u32() as u16);

        ///////////////////////////////////////////////////////////////////////////////////////////////////
        // testing permute 4
        ///////////////////////////////////////////////////////////////////////////////////////////////////

        let mut set = HashSet::new();
        let mut b = a.clone();
        set.insert(b.clone());
        for _i in 0..200 {
            b = lfsr.permute_4(&b);
            set.insert(b.clone());
        }

        println!("The set is values are {:?}", set);
        assert!(set.len() == 24, "There should be 24 values");

        let mut map = HashMap::new();
        let mut b = a.clone();
        map.insert(b.clone(), 0);
        for _i in 0..240000 {
            b = lfsr.permute_4(&b);
            if map.contains_key(&b) {
                map.insert(b.clone(), map.get(&b).unwrap() + 1);
            } else {
                map.insert(b.clone(), 0);
            }
        }

        println!("The lfsr map values are {:?}", map);
        assert!(map.len() == 24, "There should be 24 values");

        ///////////////////////////////////////////////////////////////////////////////////////////////////
        // testing permute with chacha
        ///////////////////////////////////////////////////////////////////////////////////////////////////

        let mut map = HashMap::new();
        let mut b = a.clone();
        map.insert(b.clone(), 0);
        for _i in 0..240000 {
            b = walksman_permutation_4(
                &b,
                &vec![
                    (rng.next_u32() % 2 == 1),
                    (rng.next_u32() % 2 == 1),
                    (rng.next_u32() % 2 == 1),
                    (rng.next_u32() % 2 == 1),
                    (rng.next_u32() % 2 == 1),
                ],
            );
            if map.contains_key(&b) {
                map.insert(b.clone(), map.get(&b).unwrap() + 1);
            } else {
                map.insert(b.clone(), 0);
            }
        }

        println!("The walksman map values are {:?}", map);
        assert!(map.len() == 24, "There should be 24 values");
    }

    #[test]
    fn test_permute_8() {
        const DUMP: bool = false;
        ///////////////////////////////////////////////////////////////////////////////////////////////////
        // base test
        ///////////////////////////////////////////////////////////////////////////////////////////////////
        let a: [usize; 8] = [1, 2, 3, 4, 5, 6, 7, 8];
        let b = walksman_permutation_8(
            &a,
            vec![
                true, false, false, false, false, false, false, false, false, false, false, false,
                false, false, false, false, false,
            ],
        );
        println!("The actual values are {:?} -> {:?}", a, b);
        assert!(b == [2, 1, 3, 4, 5, 6, 7, 8]);

        let mut rng = ChaCha8Rng::from_os_rng();

        let mut lfsr = LFSR16::new(rng.next_u32() as u16);
        ///////////////////////////////////////////////////////////////////////////////////////////////////
        // Testing the permutation and shuffle for size 8
        ///////////////////////////////////////////////////////////////////////////////////////////////////

        let mut map = HashMap::new();
        let mut b = a.clone();
        map.insert(b.clone(), 0);
        for i in 0..(1 << 20) {
            let mut control = vec![];
            for index in 0..17 {
                control.push(((i >> index) & 1) == 1);
            }
            b = lfsr.riffle_array(&b);
            b = walksman_permutation_8(&b, control);
            if map.contains_key(&b) {
                map.insert(b.clone(), map.get(&b).unwrap() + 1);
            } else {
                map.insert(b.clone(), 0);
            }
        }

        println!(
            "The walksman 8 map values are {:?}",
            map.iter()
                .filter(|(_, v)| **v > 20)
                .map(|(_, v)| *v)
                .collect::<Vec<usize>>()
                .len()
        );
        assert!(map.len() == 40320, "There should be 40320 values");

        ///////////////////////////////////////////////////////////////////////////////////////////////////
        // Testing the permutation and shuffle for size 8
        ///////////////////////////////////////////////////////////////////////////////////////////////////
        
        #[derive(Clone, Debug)]
        struct Perm {
            frequency: usize,
            controls: Vec<Vec<bool>>,
        }

        let mut map: HashMap<[usize;8], Perm> = HashMap::new();
        // map.insert(b.clone(), 0);
        for i in 0..(1 << 17) {
            let mut control = vec![];
            for index in 0..17 {
                control.push(((i >> index) & 1) == 1);
            }
            // b = lfsr.riffle_array(&b);
            b = walksman_permutation_8(&a, control.clone());
            if map.contains_key(&b) {
                let mut new_perm: Perm = map.get(&b).unwrap().clone();
                new_perm.frequency += 1;
                new_perm.controls.push(control.clone());
                map.insert(b.clone(), new_perm);
            } else {
                let mut store_of_controls = vec![];
                store_of_controls.push(control.clone());
                map.insert(b.clone(), Perm{
                    frequency: 1,
                    controls: store_of_controls,
                });
            }
        }

        println!(
            "The walksman 8 with same input map values are {:?}",
            map.iter()
            .filter(|(_, v)| v.frequency == 32)
            .map(|(k, v)| (k.clone(), v.controls.iter().map(|x|{
                let number_of_true = x.iter().filter(|&&b| b).count();
                (number_of_true, (x.len() - number_of_true))
            }).collect::<Vec<(usize, usize)>>()))
            .collect::<Vec<([usize;8],Vec<(usize, usize)>)>>()[0]
            // .len()
        );
        assert!(map.len() == 40320, "There should be 40320 values");

        ///////////////////////////////////////////////////////////////////////////////////////////////////
        // testing the in built shuffle function
        ///////////////////////////////////////////////////////////////////////////////////////////////////

        let mut map = HashMap::new();
        let mut b = a.clone();
        map.insert(b.clone(), 0);
        for _ in 0..(1 << 20) {
            b.shuffle(&mut rng);
            if map.contains_key(&b) {
                map.insert(b.clone(), map.get(&b).unwrap() + 1);
            } else {
                map.insert(b.clone(), 0);
            }
        }

        println!(
            "The walksman 8 map values are {:?}",
            map.iter()
                .filter(|(_, v)| **v > 20)
                .map(|(_, v)| *v)
                .collect::<Vec<usize>>()
                .len()
        );
        assert!(map.len() == 40320, "There should be 40320 values");

        ///////////////////////////////////////////////////////////////////////////////////////////////////
        // testing the frequncy of occurance of each value in the different positions
        // for riffle with permutation
        ///////////////////////////////////////////////////////////////////////////////////////////////////

        let mut occurance = HashMap::new();
        let mut b: [usize; 8] = [1, 2, 3, 4, 5, 6, 7, 8];
        for i in 1..=8 {
            occurance.insert(i, [0 as usize; 8]);
        }

        for i in 0..(1 << 20) {
            let mut control = vec![];
            for index in 0..17 {
                control.push(((i >> index) & 1) == 1);
            }
            b = lfsr.riffle_array(&b);
            b = walksman_permutation_8(&b, control);
            for index in 1..=8 {
                let position = b.iter().position(|&x| x == index).unwrap();
                let mut values = *occurance.get(&index).unwrap();
                values[position] += 1;
                occurance.insert(index, values);
            }
        }

        println!("The walksman 8 occurance of values are {:?}", occurance);
        if DUMP {
        // Convert to JSON string
        let json_string = serde_json::to_string_pretty(&occurance).unwrap();
        // Write to a file
        let mut file = File::create("riffle-permute.json").unwrap();
        file.write_all(json_string.as_bytes()).unwrap();

        let reduced_map: HashMap<usize, Vec<usize>> = occurance.iter().map(|(k, v)| {
            (*k, v.iter().map(|&x| x/ 10_000).collect::<Vec<usize>>())
        }).collect();

        // Convert to JSON string
        let json_string = serde_json::to_string_pretty(&reduced_map).unwrap();
        // Write to a file
        let mut file = File::create("riffle-permute-reduced.json").unwrap();
        file.write_all(json_string.as_bytes()).unwrap();
        }
        ///////////////////////////////////////////////////////////////////////////////////////////////////
        // testing the frequncy of occurance of each value in the different positions
        // for in built shuffle
        ///////////////////////////////////////////////////////////////////////////////////////////////////

        let mut occurance = HashMap::new();
        let mut b: [usize; 8] = [1, 2, 3, 4, 5, 6, 7, 8];
        for i in 1..=8 {
            occurance.insert(i, [0 as usize; 8]);
        }

        for _ in 0..(1 << 20) {
            b.shuffle(&mut rng);
            for index in 1..=8 {
                let position = b.iter().position(|&x| x == index).unwrap();
                let mut values = *occurance.get(&index).unwrap();
                values[position] += 1;
                occurance.insert(index, values);
            }
        }

        println!("The walksman 8 occurance of values are {:?}", occurance);
        if DUMP {
         // Convert to JSON string
         let json_string = serde_json::to_string_pretty(&occurance).unwrap();
         // Write to a file
         let mut file = File::create("rust-shuffle.json").unwrap();
         file.write_all(json_string.as_bytes()).unwrap();
        }
    }

}
