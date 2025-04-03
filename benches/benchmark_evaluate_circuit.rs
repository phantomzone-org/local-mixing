use criterion::{black_box, criterion_group, criterion_main, Criterion};
use local_mixing::circuit::Circuit;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

pub fn criterion_benchmark(c: &mut Criterion) {
    let mut rng = ChaCha8Rng::from_os_rng();
    let num_wires = 11;
    let num_gates = 5;
    let circuit = Circuit::random(num_wires, num_gates, &mut rng);

    c.bench_function("evaluate-circuit", |b| {
        b.iter(|| {
            black_box(
                (0..1 << num_wires)
                    .map(|i| {
                        let mut input = i;
                        circuit.gates.iter().for_each(|g| {
                            let a = (input & (1 << g.wires[1])) != 0;
                            let b = (input & (1 << g.wires[2])) != 0;
                            let x = g.evaluate_cf(a, b);
                            input ^= (x as usize) << g.wires[0];
                        });
                        input
                    })
                    .collect::<Vec<usize>>(),
            )
        });
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
