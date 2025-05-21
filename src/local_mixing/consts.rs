use crate::circuit::cf::GateControlFunc;

/// Inflationary stage input size
pub const N_OUT_INF: usize = 2;
/// Inflationary stage replacement size
pub const N_IN_INF: usize = 4;
/// Kneading stage input size
pub const N_OUT_KND: usize = 4;
/// Kneading stage replacement size
pub const N_IN_KND: usize = 4;

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

/// Number of steps between every save
pub const EPOCH_SIZE: usize = 10000;

#[cfg(feature = "correctness")]
/// Correctness check iternations
pub const CORRECTNESS_CHECK_ITER: usize = 1000;
