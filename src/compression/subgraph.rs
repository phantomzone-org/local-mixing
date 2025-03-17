use crate::circuit::Gate;

use super::ct::CompressionTable;

pub type DependencyData = Vec<Vec<bool>>;

pub fn dependency_data(gates: &[Gate]) -> DependencyData {
    let n = gates.len();
    let mut dependency_data = vec![vec![false; n]; n];

    for i in 0..n {
        for j in i + 1..n {
            if gates[i].collides_with(&gates[j]) {
                dependency_data[i][j] = true;
                continue;
            }
            for k in i + 1..j {
                if dependency_data[i][k] && gates[k].collides_with(&gates[j]) {
                    dependency_data[i][j] = true;
                    break;
                }
            }
        }
    }

    dependency_data
}

pub fn enumerate_subgraphs(
    gates: &[Gate],
    dependency_data: &DependencyData,
    subset_size: usize,
    slice_size: usize,
    ct: &CompressionTable<4, 4, 16>,
) -> Option<(Vec<usize>, Vec<Gate>)> {
    let n = gates.len();

    for i in 0..n {
        let x = vec![i];
        let y: Vec<usize> = (i + 1..n).collect();
        let res = df(gates, dependency_data, &x, &y, subset_size, slice_size, ct);
        if res.is_some() {
            return res;
        }
    }

    None
}

fn df(
    gates: &[Gate],
    dependency_data: &DependencyData,
    x: &Vec<usize>,
    y: &Vec<usize>,
    subset_size: usize,
    slice_size: usize,
    ct: &CompressionTable<4, 4, 16>,
) -> Option<(Vec<usize>, Vec<Gate>)> {
    if x.len() == subset_size {
        for index in 0..subset_size - slice_size + 1 {
            let idx = &x[index..index + slice_size];
            let subcircuit = idx.iter().map(|&i| gates[i]).collect();
            if let Some(replacement) = ct.compress_circuit(&subcircuit) {
                let mut optimized: Vec<Gate> = x.iter().map(|&i| gates[i]).collect();
                let replacement_len = replacement.len();
                optimized.splice(index..index + slice_size, replacement);
                let mut new_index = index + replacement_len;
                while slice_size < optimized.len() && new_index < optimized.len() - slice_size + 1 {
                    if let Some(replacement) =
                        ct.compress_circuit(&optimized[new_index..new_index + slice_size].to_vec())
                    {
                        let replacement_len = replacement.len();
                        optimized.splice(new_index..new_index + slice_size, replacement);
                        new_index += replacement_len;
                    } else {
                        new_index += 1;
                    }
                }

                return Some((x.to_vec(), optimized));
            }
        }

        return None;
    }

    let mut y = y.clone();

    let mut a = vec![];
    for i in 0..y.len() {
        for j in 0..x.len() {
            if x[j] >= y[i] {
                break;
            }
            if gates[x[j]].collides_with(&gates[y[i]]) {
                a.push(y[i]);
                break;
            }
        }
    }

    let mut a_ctr = 0;
    while a_ctr < a.len() {
        let v = a[a_ctr];
        let mut eval_x = x.clone();
        let pos = eval_x.binary_search(&v).unwrap_or_else(|e| e);
        eval_x.insert(pos, v);

        let mut eval_y = y.clone();
        for i in 0..eval_y.len() {
            if eval_y[i] == v {
                eval_y.remove(i);
                break;
            }
        }

        let res = df(
            gates,
            dependency_data,
            &eval_x,
            &eval_y,
            subset_size,
            slice_size,
            ct,
        );
        if res.is_some() {
            return res;
        }

        a.retain(|&i| !dependency_data[v][i]);
        y.retain(|&i| !dependency_data[v][i]);

        a_ctr += 1;
    }

    let mut b = vec![];
    for i in (0..y.len()).rev() {
        for j in (0..x.len()).rev() {
            if x[j] <= y[i] {
                break;
            }
            if gates[x[j]].collides_with(&gates[i]) {
                b.push(y[i]);
                break;
            }
        }
    }

    let mut b_ctr = 0;
    while b_ctr < b.len() {
        let v = b[b_ctr];
        let mut eval_x = x.clone();
        let pos = eval_x.binary_search(&v).unwrap_or_else(|e| e);
        eval_x.insert(pos, v);

        let mut eval_y = y.clone();
        for i in 0..eval_y.len() {
            if eval_y[i] == v {
                eval_y.remove(i);
                break;
            }
        }

        let res = df(
            gates,
            dependency_data,
            &eval_x,
            &eval_y,
            subset_size,
            slice_size,
            ct,
        );
        if res.is_some() {
            return res;
        }

        b.retain(|&i| !dependency_data[i][v]);
        y.retain(|&i| !dependency_data[i][v]);

        b_ctr += 1;
    }

    None
}

pub fn permute_around_subgraph(
    gates: &[Gate],
    subgraph: &Vec<usize>,
    dependency_data: &DependencyData,
) -> (Vec<Gate>, usize) {
    let mut permuted = gates.to_vec();

    let mut to_before = vec![];
    let mut to_after = vec![];

    for i in 0..subgraph.len() - 1 {
        for j in subgraph[i] + 1..subgraph[i + 1] {
            // if dependency_data[i][j] {
            //     to_after.push(gates[j]);
            // } else {
            //     to_before.push(gates[j]);
            // }

            if (0..=i).any(|id| dependency_data[subgraph[id]][j]) {
                to_after.push(gates[j]);
            } else {
                to_before.push(gates[j]);
            }
        }
    }

    let mut index = subgraph[0];
    for gate in &to_before {
        permuted[index] = *gate;
        index += 1;
    }

    for &subgraph_gate_index in subgraph {
        permuted[index] = gates[subgraph_gate_index];
        index += 1;
    }

    for gate in &to_after {
        permuted[index] = *gate;
        index += 1;
    }

    (permuted, subgraph[0] + to_before.len())
}

#[cfg(test)]
mod test {
    use crate::{circuit::Circuit, compression::ct::fetch_or_create_compression_table};

    use super::{dependency_data, enumerate_subgraphs};

    #[test]
    fn test_enumerate_subgraph() {
        let ct = fetch_or_create_compression_table();
        let c = Circuit::random(64, 20, &mut rand::rng()).gates;
        let dd = dependency_data(&c);
        enumerate_subgraphs(&c, &dd, 10, 10, &ct);
    }
}
