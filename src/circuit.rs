use rand::{Rng, RngCore};

#[derive(Clone, Copy, Debug)]
pub struct Gate {
    pub target: u32,
    pub control: [u32; 2],
    control_func: u8,
}

impl Gate {
    pub fn collides_with(&self, other: &Self) -> bool {
        self.target == other.control[0]
            || self.target == other.control[1]
            || other.target == self.control[0]
            || other.target == self.control[1]
    }
}

#[derive(Clone, Debug)]
pub struct Circuit {
    pub gates: Vec<Gate>,
}

impl Circuit {
    pub fn random<R: RngCore>(num_wires: u32, num_gates: usize, rng: &mut R) -> Self {
        let mut gates = vec![];
        for _ in 0..num_gates {
            loop {
                let target = rng.gen_range(0..num_wires);
                let control_one = rng.gen_range(0..num_wires);
                let control_two = rng.gen_range(0..num_wires);

                if target != control_one && target != control_two && control_one != control_two {
                    gates.push(Gate {
                        target,
                        control: [control_one, control_two],
                        control_func: rng.gen_range(0..16),
                    });
                    break;
                }
            }
        }

        Self { gates }
    }
}

#[cfg(test)]
mod tests {
    use rand::thread_rng;

    use super::*;

    #[test]
    fn test_random_circuit() {
        let mut rng = thread_rng();
        let circuit = Circuit::random(10, 50, &mut rng);
        assert!(circuit.gates.len() == 50);
    }
}
