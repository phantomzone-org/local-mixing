use std::collections::HashSet;

use crate::circuit::cf::Base2GateControlFunc;

use super::tracer::{ReplacementFailFields, ReplacementSampleFields};

#[derive(Debug)]
pub enum SuccessCase {
    IdentitySubcircuits,
    NegatedCFs,
    Other,
}

#[derive(Debug)]
pub enum FailCase {
    AllDistinctTargets,
    Other,
}

pub fn classify_success(replacement: &ReplacementSampleFields) -> Vec<SuccessCase> {
    let mut classifications = vec![];

    // IdentitySubcircuits
    let mut input_identity_subcircuit_idx = vec![false; replacement.input.len()];
    for i in 0..replacement.input.len() {
        for j in i + 1..replacement.input.len() {
            if replacement.input[i] == replacement.input[j] {
                input_identity_subcircuit_idx[i] = true;
                input_identity_subcircuit_idx[j] = true;
            }
        }
    }
    let mut output_identity_subcircuit_idx = vec![false; replacement.output.len()];
    for i in 0..replacement.output.len() {
        for j in i + 1..replacement.output.len() {
            if replacement.output[i] == replacement.output[j] {
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
    let mut input_negated_cf_idx = vec![false; replacement.input.len()];
    let mut output_negated_cf_idx = vec![false; replacement.output.len()];
    for i in 0..replacement.input.len() {
        for j in 0..replacement.output.len() {
            let g1 = replacement.input[i];
            let g2 = replacement.output[j];
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
    for i in 0..replacement.input.len() {
        if !input_identity_subcircuit_idx[i]
            && !input_negated_cf_idx[i]
            && !replacement.output.contains(&replacement.input[i])
        {
            classifications.push(SuccessCase::Other);
            return classifications;
        }
    }

    classifications
}

pub fn classify_fail(circuit: &ReplacementFailFields) -> FailCase {
    let target_set: HashSet<_> = circuit.circuit.iter().map(|&g| g.2).collect();
    if target_set.len() == circuit.circuit.len() {
        return FailCase::AllDistinctTargets;
    }

    FailCase::Other
}
