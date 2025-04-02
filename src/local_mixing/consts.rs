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

/// Default number of gates for new circuits
pub const DEFAULT_NUM_GATES: usize = 1000;

#[cfg(feature = "correctness")]
/// Correctness check iternations
pub const CORRECTNESS_CHECK_ITER: usize = 1000;
