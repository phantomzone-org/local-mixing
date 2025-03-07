use rand_chacha::ChaCha8Rng;
use rand::{seq::SliceRandom, RngCore, SeedableRng};
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


mod tests {
    use std::collections::{HashMap, HashSet};

    use rand::Rng;
    use rand_chacha::ChaCha8Rng;

    use std::fs::File;
    use std::io::Write;
    use serde_json;

    use crate::replacement::lfsr::LFSR16;

    use super::*;
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
        // // testing the in built shuffle function
        // ///////////////////////////////////////////////////////////////////////////////////////////////////

        // let mut map = HashMap::new();
        // let mut b = a.clone();
        // map.insert(b.clone(), 0);
        // for _ in 0..(1 << 20) {
        //     b.shuffle(&mut rng);
        //     if map.contains_key(&b) {
        //         map.insert(b.clone(), map.get(&b).unwrap() + 1);
        //     } else {
        //         map.insert(b.clone(), 0);
        //     }
        // }

        // println!(
        //     "The walksman 8 map values are {:?}",
        //     map.iter()
        //         .filter(|(_, v)| **v > 20)
        //         .map(|(_, v)| *v)
        //         .collect::<Vec<usize>>()
        //         .len()
        // );
        // assert!(map.len() == 40320, "There should be 40320 values");

        // ///////////////////////////////////////////////////////////////////////////////////////////////////
        // // testing the frequncy of occurance of each value in the different positions
        // // for riffle with permutation
        // ///////////////////////////////////////////////////////////////////////////////////////////////////

        let mut occurance = HashMap::new();
        let mut b: [usize; 5] = [1, 2, 3, 4, 5];
        for i in 1..=5 {
            occurance.insert(i, [0 as usize; 5]);
        }

        for i in 0..(1 << 25) {
            let mut control = vec![];
            for index in 0..17 {
                control.push(((i >> index) & 1) == 1);
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
}
