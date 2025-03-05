use crate::circuit::{
    analysis::{projection_circuit, truth_table},
    cf::Base2GateControlFunc,
    Gate,
};

pub const PROJ_WIRES: usize = 9;
pub const PROJ_TT_SIZE: usize = 1 << 9;
pub const ALL_GATE_WIRES: [[u32; 3]; PROJ_WIRES * (PROJ_WIRES - 1) * (PROJ_WIRES - 2)] = {
    let mut table = [[0; 3]; PROJ_WIRES * (PROJ_WIRES - 1) * (PROJ_WIRES - 2)];
    let mut ctr = 0;
    let mut t = 0;
    while t < 9 {
        let mut c1 = 0;
        while c1 < 9 {
            let mut c2 = 0;
            while c2 < 9 {
                if t != c1 && t != c2 && c1 != c2 {
                    table[ctr] = [t, c1, c2];
                    ctr += 1;
                }
                c2 += 1;
            }
            c1 += 1;
        }
        t += 1;
    }

    table
};

pub fn populate_rainbow_table_brute_force<const SIZE: usize>() {
    assert!(SIZE >= 1);
    match SIZE {
        1 => populate_rainbow_table_size_one(),
        _ => {
            // Gate 1 always has wires = [0, 1, 2]
            let mut current_circuit = [Gate {
                wires: [0, 0, 0],
                control_func: 0,
            }; SIZE];
            current_circuit[0].wires = [0, 1, 2];
            populate_rainbow_table_recursive::<SIZE>(1, &mut current_circuit);
        }
    }
}

fn populate_rainbow_table_recursive<const SIZE: usize>(
    current_size: usize,
    current_circuit: &mut [Gate; SIZE],
) {
    if current_size == SIZE {
        let (mut proj_circuit, _) = projection_circuit::<SIZE, PROJ_WIRES>(current_circuit);
        fill_in_cfs_and_eval_recursive(0, &mut proj_circuit);
        return;
    }

    for &wires in ALL_GATE_WIRES.iter() {
        current_circuit[current_size] = Gate {
            wires,
            control_func: 0,
        };
        populate_rainbow_table_recursive(current_size + 1, current_circuit);
    }
}

fn fill_in_cfs_and_eval_recursive<const SIZE: usize>(
    current_size: usize,
    proj_circuit: &mut [Gate; SIZE],
) {
    if current_size == SIZE {
        let _tt: [u32; PROJ_TT_SIZE] = truth_table(&proj_circuit);
        return;
    }

    for cf in 0..Base2GateControlFunc::COUNT {
        proj_circuit[current_size].control_func = cf;
        fill_in_cfs_and_eval_recursive(current_size + 1, proj_circuit);
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
