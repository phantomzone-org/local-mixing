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

/// Precompiled table of candidate replacement gates over projection wires
pub const PROJ_GATE_CANDIDATES_SIZE: usize = N_PROJ_WIRES * (N_PROJ_WIRES - 1) * (N_PROJ_WIRES - 2);
pub const PROJ_GATE_CANDIDATES: [[u32; 3]; PROJ_GATE_CANDIDATES_SIZE] = {
    let mut candidates = [[0; 3]; PROJ_GATE_CANDIDATES_SIZE];
    let mut index = 0;
    let n = N_PROJ_WIRES;
    let mut i = 0;
    while i < n {
        let mut j = 0;
        while j < n {
            if j != i {
                let mut k = 0;
                while k < n {
                    if k != i && k != j {
                        candidates[index] = [i as u32, j as u32, k as u32];
                        index += 1;
                    }
                    k += 1;
                }
            }
            j += 1;
        }
        i += 1;
    }
    candidates
};

pub const PROJ_GATE_CANDIDATES_ONE_WIRE: [[[u32; 2]; (N_PROJ_WIRES - 1) * (N_PROJ_WIRES - 2)];
    N_PROJ_WIRES] = {
    let mut candidates = [[[0; 2]; (N_PROJ_WIRES - 1) * (N_PROJ_WIRES - 2)]; N_PROJ_WIRES];
    let n = N_PROJ_WIRES;
    let mut i = 0;
    while i < n {
        let mut index = 0;
        let mut j = 0;
        while j < n {
            if j != i {
                let mut k = 0;
                while k < n {
                    if k != i && k != j {
                        candidates[i][index] = [j as u32, k as u32];
                        index += 1;
                    }
                    k += 1;
                }
            }
            j += 1;
        }
        i += 1;
    }
    candidates
};
