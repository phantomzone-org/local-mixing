use std::collections::{HashMap, HashSet};

use rand::Rng;

use crate::circuit::Gate;

#[derive(Clone, Debug)]
pub struct DependencyData {
    index: usize,
    prev_dependencies: Vec<usize>,
    depend_on_future: Option<usize>,
}

pub fn create_dependency_graph(gates: &Vec<Gate>) -> Vec<DependencyData> {
    let mut dependency_graph = vec![];
    let mut variable_depended_last: HashMap<usize, usize> = HashMap::new();
    let mut variable_set_last: HashMap<usize, usize> = HashMap::new();

    gates.iter().enumerate().for_each(|(i, g)| {
        let target = g.wires[0];
        let controls = [g.wires[1], g.wires[2]];
        let target_depends = variable_depended_last.get(&target);
        let controls_depend: Vec<usize> = controls
            .iter()
            .filter_map(|&w| variable_set_last.get(&w).copied())
            .collect();
        let depend_on_future = gates[i + 1..]
            .iter()
            .position(|future_gate| {
                let future_vars = &future_gate.wires;
                let future_target = future_vars[0];
                // TODO: consider allowing equal target/control
                future_vars.contains(&target) || controls.contains(&future_target)
            })
            .map(|pos| pos + i + 1);

        let mut prev_dependencies = vec![];
        if let Some(&t) = target_depends {
            prev_dependencies.push(t);
        }
        for c in controls_depend {
            prev_dependencies.push(c);
        }

        dependency_graph.push(DependencyData {
            index: i,
            prev_dependencies,
            depend_on_future,
        });

        g.wires.iter().for_each(|&w| {
            variable_depended_last.insert(w, i);
        });
        variable_set_last.insert(g.wires[0], i);
    });

    dependency_graph
}

pub fn dependency_graph_to_map(dg: &Vec<DependencyData>) -> HashMap<usize, Vec<usize>> {
    let mut d_map: HashMap<usize, Vec<usize>> = HashMap::new();

    for d in dg {
        for depend in &d.prev_dependencies {
            d_map.entry(*depend).or_insert_with(Vec::new).push(d.index);
        }
    }

    d_map
}

pub fn dependency_shuffle<R: Rng>(d: &Vec<DependencyData>, rng: &mut R) -> Vec<DependencyData> {
    let mut shuffled = vec![];
    let mut curr = d.len();

    while curr != 0 {
        let random_index = rng.random_range(0..curr);
        curr -= 1;

        shuffled.push(d[random_index].clone());
    }

    shuffled
}

pub fn dependency_sort(
    d_map: &HashMap<usize, Vec<usize>>,
    node: &DependencyData,
    max_length: usize,
    nodes: &Vec<DependencyData>,
    gates: &Vec<Gate>,
) -> Vec<usize> {
    let mut current_subset = vec![];
    let mut current_subset_set = HashSet::new();
    let mut non_explored_dependencies = vec![]; // sorted in place

    current_subset.push(node.index);
    current_subset_set.insert(node.index);
    for i in &node.prev_dependencies {
        if let Err(pos) = non_explored_dependencies.binary_search(i) {
            non_explored_dependencies.insert(pos, *i);
        }
    }
    loop {
        if non_explored_dependencies.len() == 0 {
            break;
        }
        let candidate = non_explored_dependencies.remove(0);
        if current_subset_set.contains(&candidate) {
            continue;
        }
        let (past_depend, add_node) = need_to_add_as_well(
            d_map,
            &current_subset,
            &current_subset_set,
            candidate,
            node.index,
            nodes,
            gates,
        );
        if past_depend.len() > 0 {
            if let Err(pos) = non_explored_dependencies.binary_search(&candidate) {
                non_explored_dependencies.insert(pos, candidate);
            }
            for i in &past_depend {
                if let Err(pos) = non_explored_dependencies.binary_search(i) {
                    non_explored_dependencies.insert(pos, *i);
                }
            }
            continue;
        }
        if add_node != candidate {
            if let Err(pos) = non_explored_dependencies.binary_search(&candidate) {
                non_explored_dependencies.insert(pos, candidate);
            }
        }
        current_subset.push(add_node);
        if current_subset.len() >= max_length {
            break;
        }
        current_subset_set.insert(add_node);
        nodes[add_node]
            .prev_dependencies
            .iter()
            .filter(|&i| !current_subset_set.contains(i))
            .for_each(|i| {
                if let Err(pos) = non_explored_dependencies.binary_search(i) {
                    non_explored_dependencies.insert(pos, *i);
                }
            });
    }

    current_subset.reverse();
    current_subset
}

pub fn need_to_add_as_well(
    d_map: &HashMap<usize, Vec<usize>>,
    subset: &Vec<usize>,
    subset_set: &HashSet<usize>,
    node: usize,
    max_node_number: usize,
    nodes: &Vec<DependencyData>,
    gates: &Vec<Gate>,
) -> (Vec<usize>, usize) {
    let mut future_depends_cache = HashMap::new();
    let mut current_node = node;
    loop {
        let requirements = get_all_future_dependencies(
            &mut future_depends_cache,
            &nodes[current_node],
            subset_set,
            max_node_number,
            gates,
        );
        if requirements.len() == 0 {
            let depends = d_map.get(&current_node).cloned().unwrap_or_else(Vec::new);
            let past_depend = depends
                .iter()
                .filter(|&i| !subset.contains(i) && *i < max_node_number)
                .copied()
                .collect();
            return (past_depend, current_node);
        } else {
            let l = requirements.len();
            current_node = requirements[l - 1];
        }
    }
}

pub fn get_all_future_dependencies(
    future_depends_cache: &mut HashMap<usize, Vec<usize>>,
    current_node: &DependencyData,
    subset_set: &HashSet<usize>,
    max_node_number: usize,
    gates: &Vec<Gate>,
) -> Vec<usize> {
    let cache = future_depends_cache.get(&current_node.index);
    if let Some(val) = cache {
        return val.clone();
    }
    if current_node.depend_on_future.is_none() {
        return vec![];
    }
    let depend_on_future_line = current_node.depend_on_future.unwrap();
    if depend_on_future_line >= max_node_number {
        return vec![];
    }
    let curr_gate = gates[current_node.index];
    let target = curr_gate.wires[0];
    let controls = &curr_gate.wires[1..];
    let mut depends = vec![];

    for i in depend_on_future_line..max_node_number {
        if subset_set.contains(&i) {
            continue;
        }
        let future_gate = gates[i];
        if future_gate.wires.contains(&target) || controls.contains(&future_gate.wires[0]) {
            depends.push(i);
        }
    }

    future_depends_cache.insert(current_node.index, depends.clone());
    depends
}

pub fn find_convex_subsets<R: Rng>(
    subset_size: usize,
    gates: &Vec<Gate>,
    rng: &mut R,
) -> Vec<Vec<usize>> {
    let mut subsets = vec![];

    let dg = create_dependency_graph(gates);
    let d_map = dependency_graph_to_map(&dg);

    let reversed = dependency_shuffle(&dg, rng);
    for node in reversed {
        let subset = dependency_sort(&d_map, &node, subset_size, &dg, gates);
        if subset.len() >= subset_size {
            subsets.push(subset);
        }
    }

    subsets
}

#[cfg(test)]
mod test {
    use crate::{cc::subgraph::dependency_graph_to_map, circuit::Circuit};

    use super::{create_dependency_graph, dependency_shuffle, dependency_sort};

    #[test]
    fn test_create_dependency_graph() {
        let c = Circuit::random(20, 20, &mut rand::rng()).gates;

        let data = create_dependency_graph(&c);

        assert!(c.len() == data.len());

        for i in 0..c.len() {
            println!("{}, {:?}", i, c[i]);
            println!("{}, {:?}", i, data[i]);
        }
    }

    #[test]
    fn test_create_dependency_map() {
        let c = Circuit::random(20, 20, &mut rand::rng()).gates;

        let data = create_dependency_graph(&c);
        let d_map = dependency_graph_to_map(&data);

        assert!(c.len() == data.len());

        for i in 0..c.len() {
            println!("{}, {:?}", i, c[i]);
            println!("{}, {:?}", i, data[i]);
        }
        println!("d_map: {:?}", d_map);
    }

    #[test]
    fn test_dependency_sort() {
        let mut rng = rand::rng();
        // let c = Circuit::random(20, 100, &mut rng).gates;
        let c = Circuit::load_from_json("bin/killari/json/random-64-1000-kneading.json", false).gates;

        let mut dg = create_dependency_graph(&c);
        let d_map = dependency_graph_to_map(&dg);

        // for i in 0..c.len() {
        //     println!("{}: {:?}", i, c[i]);
        //     // println!("{}: {:?}", i, dg[i]);
        // }
        // println!("d_map: {:?}", d_map);

        let reversed = dependency_shuffle(&mut dg, &mut rng);

        // for i in 0..c.len() {
        //     println!("reversed {}: {:?}", i, dg[i]);
        // }
        for node in reversed {
            let subset = dependency_sort(&d_map, &node, 5, &dg, &c);
            if subset.len() >= 5 {
                println!("{:?}", subset);
            }
        }
    }
}
