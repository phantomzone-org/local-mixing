use crate::circuit::circuit::check_equiv_probabilistic;

/**
https://ieeexplore.ieee.org/abstract/document/7811247
*/
use super::{ct::CompressionTable, Gate};

pub fn enumerate_subgraphs(
    gates: &[Gate],
    succ: &Vec<Vec<usize>>,
    pred: &Vec<Vec<usize>>,
    subset_size: usize,
    slice_size: usize,
    ct: &CompressionTable<4, 4, 16>,
) -> Option<(Vec<usize>, Vec<Gate>)> {
    let n = gates.len();

    for i in 0..n {
        let res = df(
            gates,
            &succ,
            &pred,
            &vec![i],
            &mut (i + 1..n).collect(),
            subset_size,
            slice_size,
            ct,
        );
        if res.is_some() {
            return res;
        }
    }

    None
}

fn df(
    gates: &[Gate],
    succ: &Vec<Vec<usize>>,
    pred: &Vec<Vec<usize>>,
    x: &Vec<usize>,
    y: &Vec<usize>,
    subset_size: usize,
    slice_size: usize,
    ct: &CompressionTable<4, 4, 16>,
) -> Option<(Vec<usize>, Vec<Gate>)> {
    if x.len() == subset_size {
        // return ct
        //     .compress_circuit(&x.iter().map(|&i| gates[i]).collect::<Vec<_>>())
        //     .and_then(|replacement| Some((x.to_vec(), replacement)));

        for index in 0..subset_size - slice_size + 1 {
            let idx = &x[index..index + slice_size];
            let subcircuit = idx.iter().map(|&i| gates[i]).collect::<Vec<_>>();
            if let Some(replacement) = ct.compress_circuit(&subcircuit) {
                // Found a change, now modify the rest of x and return
                let mut optimized = x.iter().map(|&i| gates[i]).collect::<Vec<_>>();
                let replacement_len = replacement.len();
                optimized.splice(index..index + slice_size, replacement);
                // let mut new_index = index + replacement_len;
                // println!("starting while");
                // while slice_size < optimized.len() && new_index < optimized.len() - slice_size + 1 {
                //     // dbg!(new_index, optimized.len(), slice_size, optimized.len() - slice_size + 1);
                //     if let Some(replacement) =
                //         ct.compress_circuit(&optimized[new_index..new_index + slice_size].to_vec())
                //     {
                //         let replacement_len = replacement.len();
                //         optimized.splice(index..index + slice_size, replacement);
                //         new_index += replacement_len;
                //     } else {
                //         new_index += 1;
                //     }
                // }
                return Some((x.to_vec(), optimized));
            }
        }

        return None;
    }

    let mut y = y.clone();

    // Gates in y and immediate successors of x
    // TODO: add v + 1 to a automatically, so contiguous blocks are also included
    let mut a: Vec<usize> = y
        .iter()
        .filter(|&&i| x.iter().any(|&j| i > j && collides(&gates[i], &gates[j])))
        .copied()
        .collect();

    let mut ctr = 0;
    while ctr < a.len() {
        let v = a[ctr];

        let mut new_x = x.clone();
        match x.binary_search(&v) {
            Ok(_) => panic!("x = {:?} should not contain v = {}", x, v),
            Err(pos) => {
                new_x.insert(pos, v);
            }
        };

        y.retain(|&i| i != v);

        let res = df(gates, succ, pred, &new_x, &y, subset_size, slice_size, ct);
        if res.is_some() {
            return res;
        }

        a.retain(|&i| !succ[v].contains(&i));
        y.retain(|&i| !succ[v].contains(&i));

        ctr += 1;
    }

    // Gates in y and immediate predecessors of x
    let mut b: Vec<usize> = y
        .iter()
        .rev()
        .filter(|&&i| x.iter().any(|&j| i < j && collides(&gates[i], &gates[j])))
        .copied()
        .collect();

    ctr = 0;
    while ctr < b.len() {
        let v = b[ctr];

        let mut new_x = x.clone();
        match x.binary_search(&v) {
            Ok(_) => panic!("x = {:?} should not contain v = {}", x, v),
            Err(pos) => {
                new_x.insert(pos, v);
            }
        };

        y.retain(|&i| i != v);

        let res = df(gates, succ, pred, &new_x, &y, subset_size, slice_size, ct);
        if res.is_some() {
            return res;
        }

        b.retain(|&i| !pred[v].contains(&i));
        y.retain(|&i| !pred[v].contains(&i));

        ctr += 1;
    }

    None
}

fn collides(g1: &Gate, g2: &Gate) -> bool {
    g1.wires[0] == g2.wires[1]
        || g1.wires[0] == g2.wires[2]
        || g2.wires[0] == g1.wires[1]
        || g2.wires[0] == g1.wires[2]
}

pub fn successors_predecessors(gates: &[Gate]) -> (Vec<Vec<usize>>, Vec<Vec<usize>>) {
    let n = gates.len();
    let mut succ = vec![vec![]; n];
    let mut pred = vec![vec![]; n];

    for i in 0..n {
        for j in i + 1..n {
            if collides(&gates[i], &gates[j])
                || succ[i].iter().any(|&k| collides(&gates[k], &gates[j]))
            {
                succ[i].push(j);
                pred[j].push(i);
            }
        }
    }

    (succ, pred)
}

#[cfg(test)]
mod test {
    use crate::{
        circuit::Circuit,
        compression::{ct::fetch_or_create_compression_table, subgraph::successors_predecessors},
    };

    #[test]
    fn test_succ_pred() {
        let c = Circuit::random(100, 1000, &mut rand::rng());
        let ct = fetch_or_create_compression_table();
        dbg!(&c);

        let (succ, pred) = successors_predecessors(&c.gates);
        let res = super::enumerate_subgraphs(&c.gates, &succ, &pred, 10, 5, &ct);
        dbg!(res);
    }
}
