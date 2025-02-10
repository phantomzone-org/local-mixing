use criterion::{black_box, criterion_group, criterion_main, Criterion};
use local_mixing::{
    circuit::Gate,
    replacement::{find_replacement_circuit, strategy::ReplacementStrategy},
};
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

pub fn criterion_benchmark(c: &mut Criterion) {
    let mut rng = ChaCha8Rng::from_os_rng();
    let circuit = [
        Gate {
            wires: [0, 1, 2],
            control_func: 3,
        },
        Gate {
            wires: [1, 3, 4],
            control_func: 9,
        },
    ];

    c.bench_function("replacement", |b| {
        b.iter(|| {
            black_box(find_replacement_circuit(
                &circuit,
                20,
                1_000_000_000,
                ReplacementStrategy::SampleActive0,
                &mut rng,
            ))
        });
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
