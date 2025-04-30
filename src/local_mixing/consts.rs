use crate::circuit::cf::GateControlFunc;

/// Size of replaced circuits (inflationary stage)
pub const N_OUT_INF: usize = 2;
/// Size of replaced circuits (kneading stage)
pub const N_OUT_KND: usize = 4;
/// Size of replacements
pub const N_IN: usize = 4;
/// Number of wires considered during replacement
pub const N_PROJ_WIRES: usize = N_IN * 2 + 1;
/// 2 ^ # projection wires
pub const N_PROJ_INPUTS: usize = 1 << N_PROJ_WIRES;

/// Input-output control functions
pub const CONTROL_FUNC_TABLE: [bool; 64] = {
    let mut table = [false; 64];
    let mut i = 0;
    while i < 64 {
        let control_func = GateControlFunc::from_u8((i >> 2) as _);
        let a = (i >> 1) & 1 == 1;
        let b = i & 1 == 1;
        table[i] = control_func.evaluate(a, b);
        i += 1
    }
    table
};

/// All possible gate bitline labelings, with control1 < control2
pub const ALL_BITLINES: [[usize; 3]; N_PROJ_WIRES * (N_PROJ_WIRES - 1) * (N_PROJ_WIRES - 2) / 2] = {
    let mut bitlines = [[0; 3]; N_PROJ_WIRES * (N_PROJ_WIRES - 1) * (N_PROJ_WIRES - 2) / 2];
    let mut i = 0;

    let mut t = 0;
    while t < N_PROJ_WIRES {
        let mut c1 = 0;
        while c1 < N_PROJ_WIRES {
            let mut c2 = c1 + 1;
            while c2 < N_PROJ_WIRES {
                if t != c1 && t != c2 {
                    bitlines[i] = [t, c1, c2];
                    i += 1;
                }
                c2 += 1;
            }
            c1 += 1;
        }
        t += 1;
    }

    bitlines
};

/// Default number of gates for auto-gen circuits
pub const DEFAULT_NUM_GATES: usize = 1000;
/// Default number of wires for auto-gen circuits
pub const DEFAULT_NUM_WIRES: usize = 64;

/// Number of steps between every save
pub const EPOCH_SIZE: usize = 10000;

#[cfg(feature = "correctness")]
/// Correctness check iternations
pub const CORRECTNESS_CHECK_ITER: usize = 1000;
