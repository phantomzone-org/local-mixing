use crate::circuit::{cf::Base2GateControlFunc, Gate};
use std::collections::HashMap;

pub struct CompressionTable<
    const MAX_SIZE: usize,
    const MAX_WIRES_USED: usize,
    const TT_SIZE: usize,
> {
    ct: HashMap<[u32; TT_SIZE], (usize, Vec<Gate>)>,
}

impl<const MAX_SIZE: usize, const MAX_WIRES_USED: usize, const TT_SIZE: usize>
    CompressionTable<MAX_SIZE, MAX_WIRES_USED, TT_SIZE>
{
    pub fn new() -> Self {
        Self {
            ct: build_rainbow_table::<MAX_SIZE, MAX_WIRES_USED, TT_SIZE>(),
        }
    }

    pub fn from_file(path: String) -> Self {
        Self {
            ct: load_rt_from_file(path),
        }
    }

    pub fn save_to_file(&self, path: String) {
        save_rt_to_file(&self.ct, path);
    }

    // pub fn
}

pub fn build_rainbow_table<
    const MAX_SIZE: usize,
    const MAX_WIRES_USED: usize,
    const TT_SIZE: usize,
>() -> HashMap<[u32; TT_SIZE], (usize, Vec<Gate>)> {
    assert!(MAX_SIZE >= 1);

    let mut ct = HashMap::new();
    // Identity gate
    ct.insert(std::array::from_fn(|i| i as u32), (0, vec![]));

    let mut current_circuit = [Gate::default(); MAX_SIZE];
    current_circuit[0].wires = [0, 1, 2];

    for cf in 1..Base2GateControlFunc::COUNT {
        current_circuit[0].control_func = cf;
        build_rainbow_table_recursive::<MAX_SIZE, MAX_WIRES_USED, TT_SIZE>(
            1,
            &mut current_circuit,
            3,
            &mut ct,
        );
    }

    ct
}

fn build_rainbow_table_recursive<
    const MAX_SIZE: usize,
    const MAX_WIRES_USED: usize,
    const TT_SIZE: usize,
>(
    current_size: usize,
    current_circuit: &mut [Gate; MAX_SIZE],
    wires_used: u32,
    ct: &mut HashMap<[u32; TT_SIZE], (usize, Vec<Gate>)>,
) {
    let tt: [u32; TT_SIZE] = std::array::from_fn(|i| {
        let mut input = i as u32;
        current_circuit.iter().take(current_size).for_each(|g| {
            let a = (input & (1 << g.wires[1])) != 0;
            let b = (input & (1 << g.wires[2])) != 0;
            let x = g.evaluate_cf(a, b);
            input ^= (x as u32) << g.wires[0];
        });
        input
    });
    ct.entry(tt)
        .and_modify(|e| {
            if current_size < e.0 {
                *e = (current_size, current_circuit[0..current_size].to_vec())
            }
        })
        .or_insert((current_size, current_circuit[0..current_size].to_vec()));

    if current_size == MAX_SIZE {
        return;
    }

    for cf in 1..Base2GateControlFunc::COUNT {
        current_circuit[current_size].control_func = cf;

        // Three new wires
        current_circuit[current_size].wires = [wires_used, wires_used + 1, wires_used + 2];
        build_rainbow_table_recursive::<MAX_SIZE, MAX_WIRES_USED, TT_SIZE>(
            current_size + 1,
            current_circuit,
            wires_used + 3,
            ct,
        );

        for w in 0..3 {
            let other_wires = other_two_wire_pos(w);
            // Two new wires: w is an old wire
            current_circuit[current_size].wires[other_wires[0]] = wires_used;
            current_circuit[current_size].wires[other_wires[1]] = wires_used + 1;
            for label in 0..wires_used {
                current_circuit[current_size].wires[w] = label;
                build_rainbow_table_recursive::<MAX_SIZE, MAX_WIRES_USED, TT_SIZE>(
                    current_size + 1,
                    current_circuit,
                    wires_used + 2,
                    ct,
                );
            }

            for label1 in 0..wires_used {
                for label2 in 0..wires_used {
                    if label1 != label2 {
                        // Two old wires: w is new
                        current_circuit[current_size].wires[w] = wires_used;
                        current_circuit[current_size].wires[other_wires[0]] = label1;
                        current_circuit[current_size].wires[other_wires[1]] = label2;
                        build_rainbow_table_recursive::<MAX_SIZE, MAX_WIRES_USED, TT_SIZE>(
                            current_size + 1,
                            current_circuit,
                            wires_used + 1,
                            ct,
                        );

                        // Three old wires
                        for label3 in 0..wires_used {
                            if label3 != label1 && label3 != label2 {
                                current_circuit[current_size].wires[w] = label3;
                                build_rainbow_table_recursive::<MAX_SIZE, MAX_WIRES_USED, TT_SIZE>(
                                    current_size + 1,
                                    current_circuit,
                                    wires_used,
                                    ct,
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}

const fn other_two_wire_pos(wire_pos: usize) -> [usize; 2] {
    match wire_pos {
        0 => [1, 2],
        1 => [0, 2],
        2 => [0, 1],
        _ => panic!(),
    }
}

pub fn save_rt_to_file<const TT_SIZE: usize>(
    ct: &HashMap<[u32; TT_SIZE], (usize, Vec<Gate>)>,
    path: String,
) {
    let data: HashMap<Vec<u32>, (usize, Vec<Gate>)> =
        ct.iter().map(|(tt, v)| (tt.to_vec(), v.clone())).collect();
    let serialized_data = bincode::serialize(&data).unwrap();
    std::fs::write(&path, serialized_data).unwrap();
}

pub fn load_rt_from_file<const TT_SIZE: usize>(
    path: String,
) -> HashMap<[u32; TT_SIZE], (usize, Vec<Gate>)> {
    let data: HashMap<Vec<u32>, (usize, Vec<Gate>)> =
        bincode::deserialize(&std::fs::read(&path).unwrap()).unwrap();
    data.into_iter()
        .map(|(tt, v)| {
            let mut key_array = [0u32; TT_SIZE];
            key_array.copy_from_slice(&tt);
            (key_array, v)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{build_rainbow_table, load_rt_from_file, save_rt_to_file};
    use crate::circuit::{
        analysis::{projection_circuit, truth_table},
        Gate,
    };

    #[test]
    fn test_rainbow_table_correctness() {
        let ct = build_rainbow_table::<3, 9, { 1 << 9 }>();
        for (tt, (size, ckt_vec)) in ct {
            assert_eq!(size, ckt_vec.len());
            match size {
                0 => assert_eq!(tt, std::array::from_fn(|i| i as u32)),
                1 => assert_eq!(tt, truth_table(&[ckt_vec[0]])),
                2 => assert_eq!(tt, truth_table(&[ckt_vec[0], ckt_vec[1]])),
                3 => assert_eq!(tt, truth_table(&[ckt_vec[0], ckt_vec[1], ckt_vec[2]])),
                _ => unreachable!(),
            };
        }
    }

    #[test]
    fn test_store_and_load_rt() {
        let ct = build_rainbow_table::<3, 9, { 1 << 9 }>();
        let path = "bin/test.db";
        save_rt_to_file(&ct, path.to_string());
        let new_rt = load_rt_from_file::<{ 1 << 9 }>(path.to_string());
        assert_eq!(ct, new_rt);
    }

    #[test]
    fn test_rt_match() {
        let ct = load_rt_from_file::<{ 1 << 9 }>("test.db".to_string());
        dbg!(ct.len());
        let circuit = [
            Gate {
                wires: [4, 5, 6],
                control_func: 3,
            },
            Gate {
                wires: [4, 5, 6],
                control_func: 3,
            },
        ];
        let proj_circuit: ([Gate; 2], [Option<u32>; 6]) = projection_circuit(&circuit);
        let tt: [u32; 512] = truth_table(&proj_circuit.0);
        dbg!(ct.get(&tt));
    }
}
