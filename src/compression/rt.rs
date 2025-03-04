use crate::circuit::{analysis::truth_table, cf::Base2GateControlFunc, Gate};

const PROJ_WIRES: usize = 9;
const PROJ_TT_SIZE: usize = 1 << PROJ_WIRES;

const fn other_two_wire_pos(wire_pos: usize) -> [usize; 2] {
    match wire_pos {
        0 => [1, 2],
        1 => [0, 2],
        2 => [0, 1],
        _ => panic!(),
    }
}

pub fn populate_rainbow_table<const SIZE: usize>() {
    assert!(SIZE >= 1);
    match SIZE {
        1 => populate_rainbow_table_size_one(),
        _ => {
            let mut current_circuit = [Gate::default(); SIZE];
            current_circuit[0].wires = [0, 1, 2];
            for cf in 0..Base2GateControlFunc::COUNT {
                current_circuit[0].control_func = cf;
                populate_rainbow_table_recursive(1, &mut current_circuit, 3);
            }
        }
    }
}

fn populate_rainbow_table_recursive<const SIZE: usize>(
    current_size: usize,
    current_circuit: &mut [Gate; SIZE],
    wires_used: u32,
) {
    if current_size == SIZE {
        let _tt: [u32; PROJ_TT_SIZE] = truth_table(&current_circuit);
        return;
    }

    for cf in 0..Base2GateControlFunc::COUNT {
        current_circuit[current_size].control_func = cf;

        // Three new wires
        current_circuit[current_size].wires = [wires_used, wires_used + 1, wires_used + 2];
        populate_rainbow_table_recursive(current_size + 1, current_circuit, wires_used + 3);

        for w in 0..3 {
            let other_wires = other_two_wire_pos(w);
            // Two new wires: w is an old wire
            current_circuit[current_size].wires[other_wires[0]] = wires_used;
            current_circuit[current_size].wires[other_wires[1]] = wires_used + 1;
            for label in 0..wires_used {
                current_circuit[current_size].wires[w] = label;
                populate_rainbow_table_recursive(current_size + 1, current_circuit, wires_used + 2);
            }

            // TODO: improve this
            for label1 in 0..wires_used {
                for label2 in 0..wires_used {
                    if label1 != label2 {
                        // Two old wires: w is new
                        current_circuit[current_size].wires[w] = wires_used;
                        current_circuit[current_size].wires[other_wires[0]] = label1;
                        current_circuit[current_size].wires[other_wires[1]] = label2;
                        populate_rainbow_table_recursive(
                            current_size + 1,
                            current_circuit,
                            wires_used + 1,
                        );

                        // Three old wires
                        for label3 in 0..wires_used {
                            if label3 != label1 && label3 != label2 {
                                current_circuit[current_size].wires[w] = label3;
                                populate_rainbow_table_recursive(
                                    current_size + 1,
                                    current_circuit,
                                    wires_used,
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}

fn populate_rainbow_table_size_one() {
    for control_func in 0..16 {
        let ckt = [Gate {
            wires: [0, 1, 2], // projected circuit always gives this,
            control_func,
        }];
        let _tt: [u32; PROJ_TT_SIZE] = truth_table(&ckt);
        // dbg!(tt);
    }
}
