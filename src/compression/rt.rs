use std::collections::HashMap;

use crate::circuit::{analysis::truth_table, cf::Base2GateControlFunc, Gate};

const fn other_two_wire_pos(wire_pos: usize) -> [usize; 2] {
    match wire_pos {
        0 => [1, 2],
        1 => [0, 2],
        2 => [0, 1],
        _ => panic!(),
    }
}

pub fn populate_rainbow_table<const SIZE: usize, const TT_SIZE: usize>(
) -> HashMap<[u32; TT_SIZE], [Gate; SIZE]> {
    assert!(SIZE >= 1);

    let mut rt = HashMap::new();

    let mut current_circuit = [Gate::default(); SIZE];
    current_circuit[0].wires = [0, 1, 2];

    for cf in 1..Base2GateControlFunc::COUNT {
        current_circuit[0].control_func = cf;
        populate_rainbow_table_recursive::<SIZE, TT_SIZE>(1, &mut current_circuit, 3, &mut rt);
    }

    rt
}

fn populate_rainbow_table_recursive<const SIZE: usize, const TT_SIZE: usize>(
    current_size: usize,
    current_circuit: &mut [Gate; SIZE],
    wires_used: u32,
    rt: &mut HashMap<[u32; TT_SIZE], [Gate; SIZE]>,
) {
    // if current_size == SIZE {
    //     let tt: [u32; TT_SIZE] = truth_table(&current_circuit);
    //     rt.insert(tt, *current_circuit);
    //     return;
    // }

    let tt = truth_table(&current_circuit);
    rt.insert(tt, *current_circuit);

    if current_size == SIZE {
        return;
    }

    for cf in 1..Base2GateControlFunc::COUNT {
        current_circuit[current_size].control_func = cf;

        // Three new wires
        current_circuit[current_size].wires = [wires_used, wires_used + 1, wires_used + 2];
        populate_rainbow_table_recursive::<SIZE, TT_SIZE>(
            current_size + 1,
            current_circuit,
            wires_used + 3,
            rt,
        );

        for w in 0..3 {
            let other_wires = other_two_wire_pos(w);
            // Two new wires: w is an old wire
            current_circuit[current_size].wires[other_wires[0]] = wires_used;
            current_circuit[current_size].wires[other_wires[1]] = wires_used + 1;
            for label in 0..wires_used {
                current_circuit[current_size].wires[w] = label;
                populate_rainbow_table_recursive::<SIZE, TT_SIZE>(
                    current_size + 1,
                    current_circuit,
                    wires_used + 2,
                    rt,
                );
            }

            // TODO: improve this
            for label1 in 0..wires_used {
                for label2 in 0..wires_used {
                    if label1 != label2 {
                        // Two old wires: w is new
                        current_circuit[current_size].wires[w] = wires_used;
                        current_circuit[current_size].wires[other_wires[0]] = label1;
                        current_circuit[current_size].wires[other_wires[1]] = label2;
                        populate_rainbow_table_recursive::<SIZE, TT_SIZE>(
                            current_size + 1,
                            current_circuit,
                            wires_used + 1,
                            rt,
                        );

                        // Three old wires
                        for label3 in 0..wires_used {
                            if label3 != label1 && label3 != label2 {
                                current_circuit[current_size].wires[w] = label3;
                                populate_rainbow_table_recursive::<SIZE, TT_SIZE>(
                                    current_size + 1,
                                    current_circuit,
                                    wires_used,
                                    rt,
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}
