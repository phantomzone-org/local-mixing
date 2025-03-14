use crate::circuit::{
    analysis::{optimal_projection_circuit, truth_table_sized},
    cf::Base2GateControlFunc,
    Gate,
};
use std::collections::HashMap;

pub struct CompressionTable<
    const MAX_SIZE: usize,
    const MAX_WIRES_USED: usize,
    const TT_SIZE: usize,
> {
    ct: HashMap<[u32; TT_SIZE], Vec<Gate>>,
}

impl<const MAX_SIZE: usize, const MAX_WIRES_USED: usize, const TT_SIZE: usize>
    CompressionTable<MAX_SIZE, MAX_WIRES_USED, TT_SIZE>
{
    pub fn new() -> Self {
        Self {
            ct: build_compression_table::<MAX_SIZE, MAX_WIRES_USED, TT_SIZE>(),
        }
    }

    pub fn from_file(path: &String) -> Self {
        Self {
            ct: load_ct_from_file(path),
        }
    }

    pub fn save_to_file(&self, path: &String) {
        save_ct_to_file(&self.ct, path);
    }

    pub fn compress_circuit(&self, circuit: &Vec<Gate>) -> Option<Vec<Gate>> {
        let (proj_circuit, proj_map, _, num_active_wires) = optimal_projection_circuit(circuit);
        if num_active_wires > MAX_WIRES_USED {
            return None;
        }

        let truth_table = truth_table_sized::<TT_SIZE>(&proj_circuit);

        let match_circuit = self.ct.get(&truth_table)?;

        let output_circuit = match_circuit
            .iter()
            .map(|g| {
                let mut wires = [0; 3];
                // for i in 0..3 {
                //     wires[i] = match proj_map[g.wires[i] as usize] {
                //         Some(label) => label,
                //         None => g.wires[i],
                //     }
                // }
                for i in 0..3 {
                    let match_wire = g.wires[i] as usize;
                    wires[i] = if match_wire < proj_map.len() {
                        proj_map[match_wire]
                    } else {
                        0
                    };
                }
                Gate {
                    wires,
                    control_func: g.control_func,
                }
            })
            .collect();

        Some(output_circuit)
    }
}

pub fn fetch_or_create_compression_table<
    const MAX_SIZE: usize,
    const MAX_WIRES_USED: usize,
    const TT_SIZE: usize,
>() -> CompressionTable<MAX_SIZE, MAX_WIRES_USED, TT_SIZE> {
    let path = format!(
        "bin/tables/{}-{}-{}-table.db",
        MAX_SIZE, MAX_WIRES_USED, TT_SIZE
    );

    if std::path::Path::new(&path).exists() {
        println!("Loading compression table from {}", path);
        CompressionTable::from_file(&path)
    } else {
        println!("Generating compression table");
        let table = CompressionTable::new();
        table.save_to_file(&path);
        println!("Table generation finished. Saved to {}", &path);
        table
    }
}

pub fn build_compression_table<
    const MAX_SIZE: usize,
    const MAX_WIRES_USED: usize,
    const TT_SIZE: usize,
>() -> HashMap<[u32; TT_SIZE], Vec<Gate>> {
    assert!(MAX_SIZE >= 1);

    let mut ct = HashMap::new();
    // Identity gate
    ct.insert(std::array::from_fn(|i| i as u32), vec![]);

    let mut current_circuit = [Gate::default(); MAX_SIZE];
    current_circuit[0].wires = [0, 1, 2];

    for cf in 1..Base2GateControlFunc::COUNT {
        current_circuit[0].control_func = cf;
        build_compression_table_recursive::<MAX_SIZE, MAX_WIRES_USED, TT_SIZE>(
            1,
            &mut current_circuit,
            3,
            &mut ct,
        );
    }

    ct
}

fn build_compression_table_recursive<
    const MAX_SIZE: usize,
    const MAX_WIRES_USED: usize,
    const TT_SIZE: usize,
>(
    current_size: usize,
    current_circuit: &mut [Gate; MAX_SIZE],
    wires_used: u32,
    ct: &mut HashMap<[u32; TT_SIZE], Vec<Gate>>,
) {
    if wires_used as usize > MAX_WIRES_USED {
        return;
    }

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
            if current_size < e.len() {
                *e = current_circuit[0..current_size].to_vec()
            }
        })
        .or_insert(current_circuit[0..current_size].to_vec());

    if current_size == MAX_SIZE || wires_used as usize == MAX_WIRES_USED {
        return;
    }

    for cf in 1..Base2GateControlFunc::COUNT {
        current_circuit[current_size].control_func = cf;

        // Three new wires
        current_circuit[current_size].wires = [wires_used, wires_used + 1, wires_used + 2];
        build_compression_table_recursive::<MAX_SIZE, MAX_WIRES_USED, TT_SIZE>(
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
                build_compression_table_recursive::<MAX_SIZE, MAX_WIRES_USED, TT_SIZE>(
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
                        build_compression_table_recursive::<MAX_SIZE, MAX_WIRES_USED, TT_SIZE>(
                            current_size + 1,
                            current_circuit,
                            wires_used + 1,
                            ct,
                        );

                        // Three old wires
                        for label3 in 0..wires_used {
                            if label3 != label1 && label3 != label2 {
                                current_circuit[current_size].wires[w] = label3;
                                build_compression_table_recursive::<
                                    MAX_SIZE,
                                    MAX_WIRES_USED,
                                    TT_SIZE,
                                >(
                                    current_size + 1, current_circuit, wires_used, ct
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

pub fn save_ct_to_file<const TT_SIZE: usize>(
    ct: &HashMap<[u32; TT_SIZE], Vec<Gate>>,
    path: &String,
) {
    let data: HashMap<Vec<u32>, Vec<Gate>> =
        ct.iter().map(|(tt, v)| (tt.to_vec(), v.clone())).collect();
    let serialized_data = bincode::serialize(&data).unwrap();
    std::fs::write(&path, serialized_data).unwrap();
}

pub fn load_ct_from_file<const TT_SIZE: usize>(
    path: &String,
) -> HashMap<[u32; TT_SIZE], Vec<Gate>> {
    let data: HashMap<Vec<u32>, Vec<Gate>> =
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
    use std::time::Instant;

    use rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;

    use super::{build_compression_table, load_ct_from_file, save_ct_to_file, CompressionTable};
    use crate::{
        circuit::{analysis::truth_table, circuit::check_equiv_probabilistic, Circuit, Gate},
        compression::ct::fetch_or_create_compression_table,
    };

    #[test]
    fn test_compression_table_correctness() {
        let ct = build_compression_table::<3, 9, { 1 << 9 }>();
        for (tt, ckt_vec) in ct {
            match ckt_vec.len() {
                0 => assert_eq!(tt, std::array::from_fn(|i| i as u32)),
                1 => assert_eq!(tt.to_vec(), truth_table(9, &vec![ckt_vec[0]])),
                2 => assert_eq!(tt.to_vec(), truth_table(9, &vec![ckt_vec[0], ckt_vec[1]])),
                3 => assert_eq!(
                    tt.to_vec(),
                    truth_table(9, &vec![ckt_vec[0], ckt_vec[1], ckt_vec[2]])
                ),
                _ => unreachable!(),
            };
        }
    }

    #[test]
    fn test_ct_correctness_less_variables() {
        let s = Instant::now();
        // let ct = build_compression_table::<4, 4, { 1 << 4 }>()
        let ct = CompressionTable::<4, 4, { 1 << 4 }>::new();
        let d = Instant::now() - s;
        dbg!(d);
        for (tt, ckt_vec) in ct.ct.clone() {
            let tt_vec = tt.to_vec();
            match ckt_vec.len() {
                0 => assert_eq!(tt, std::array::from_fn(|i| i as u32)),
                1 => assert_eq!(tt_vec, truth_table(4, &vec![ckt_vec[0]])),
                2 => assert_eq!(tt_vec, truth_table(4, &vec![ckt_vec[0], ckt_vec[1]])),
                3 => assert_eq!(
                    tt_vec,
                    truth_table(4, &vec![ckt_vec[0], ckt_vec[1], ckt_vec[2]])
                ),
                4 => assert_eq!(
                    tt_vec,
                    truth_table(4, &vec![ckt_vec[0], ckt_vec[1], ckt_vec[2], ckt_vec[3]])
                ),
                _ => unreachable!(),
            };
        }
        ct.save_to_file(&"test4-4.db".to_string());
    }

    #[test]
    fn test_store_and_load_ct() {
        let ct = build_compression_table::<3, 9, { 1 << 9 }>();
        let path = "bin/test.db";
        save_ct_to_file(&ct, &path.to_string());
        let new_ct = load_ct_from_file::<{ 1 << 9 }>(&path.to_string());
        assert_eq!(ct, new_ct);
    }

    #[test]
    fn test_ct_compress() {
        let ct = fetch_or_create_compression_table::<3, 9, { 1 << 9 }>();
        let circuit = vec![
            Gate {
                wires: [33, 22, 60],
                control_func: 15,
            },
            Gate {
                wires: [33, 2, 22],
                control_func: 3,
            },
            Gate {
                wires: [33, 22, 2],
                control_func: 9,
            },
            Gate {
                wires: [55, 12, 24],
                control_func: 15,
            },
        ];
        let res = ct.compress_circuit(&circuit);
        assert!(res.is_some());
        let out = res.unwrap();
        let c1 = Circuit {
            num_wires: 64,
            gates: circuit.to_vec(),
        };
        let c2 = Circuit {
            num_wires: 64,
            gates: out.to_vec(),
        };
        assert_eq!(
            Ok(()),
            check_equiv_probabilistic(&c1, &c2, 1000000, &mut ChaCha8Rng::from_os_rng())
        );
    }
}
