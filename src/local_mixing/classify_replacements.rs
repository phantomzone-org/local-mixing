use std::collections::HashSet;

use crate::circuit::{cf::Base2GateControlFunc, circuit::GateData};

#[derive(Debug)]
pub enum SuccessCase {
    IdentitySubcircuits,
    NegatedCFs,
    Other,
}

impl std::fmt::Display for SuccessCase {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let s = match self {
            SuccessCase::IdentitySubcircuits => "IdentitySubcircuits",
            SuccessCase::NegatedCFs => "NegatedCFs",
            SuccessCase::Other => "Other",
        };
        write!(f, "{}", s)
    }
}

#[derive(Debug)]
pub enum FailCase {
    AllDistinctTargets,
    Other,
}

impl std::fmt::Display for FailCase {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let s = match self {
            FailCase::AllDistinctTargets => "AllDistinctTargets",
            FailCase::Other => "Other",
        };
        write!(f, "{}", s)
    }
}

pub fn classify_success(input: &Vec<GateData>, output: &Vec<GateData>) -> Vec<SuccessCase> {
    let mut classifications = vec![];

    // IdentitySubcircuits
    let mut input_identity_subcircuit_idx = vec![false; input.len()];
    for i in 0..input.len() {
        for j in i + 1..input.len() {
            if input[i] == input[j] {
                input_identity_subcircuit_idx[i] = true;
                input_identity_subcircuit_idx[j] = true;
            }
        }
    }
    let mut output_identity_subcircuit_idx = vec![false; output.len()];
    for i in 0..output.len() {
        for j in i + 1..output.len() {
            if output[i] == output[j] {
                output_identity_subcircuit_idx[i] = true;
                output_identity_subcircuit_idx[j] = true;
            }
        }
    }
    if input_identity_subcircuit_idx.iter().any(|&b| b)
        && output_identity_subcircuit_idx.iter().any(|&b| b)
    {
        classifications.push(SuccessCase::IdentitySubcircuits);
    }

    // NegatedCFs
    let mut input_negated_cf_idx = vec![false; input.len()];
    let mut output_negated_cf_idx = vec![false; output.len()];
    for i in 0..input.len() {
        for j in 0..output.len() {
            let g1 = input[i];
            let g2 = output[j];
            if !input_identity_subcircuit_idx[i]
                && !output_identity_subcircuit_idx[j]
                && g1.wires() == g2.wires()
                && g1.cf() == Base2GateControlFunc::negated(g2.cf())
            {
                input_negated_cf_idx[i] = true;
                output_negated_cf_idx[j] = true;
            }
        }
    }

    if input_negated_cf_idx.iter().any(|&b| b) {
        classifications.push(SuccessCase::NegatedCFs);
    }

    // Other
    for i in 0..input.len() {
        if !input_identity_subcircuit_idx[i]
            && !input_negated_cf_idx[i]
            && !output.contains(&input[i])
        {
            classifications.push(SuccessCase::Other);
            return classifications;
        }
    }

    classifications
}

pub fn classify_fail(circuit: &Vec<GateData>) -> FailCase {
    let target_set: HashSet<_> = circuit.iter().map(|&g| g.2).collect();
    if target_set.len() == circuit.len() {
        return FailCase::AllDistinctTargets;
    }

    FailCase::Other
}
