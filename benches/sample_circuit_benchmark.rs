use criterion::{black_box, criterion_group, criterion_main, Criterion};
use local_mixing::{circuit::Gate, replacement::sample_random_circuit};
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

pub fn criterion_benchmark(c: &mut Criterion) {
    let mut rng = ChaCha8Rng::from_entropy();
    let mut circuit = [Gate::default(); 4];
    let active_wires = [
        [
            true, false, false, true, false, false, false, false, false, false, false,
        ],
        [
            true, true, true, false, true, false, false, false, false, false, false,
        ],
    ];

    c.bench_function("sample circuit", |b| {
        b.iter(|| {
            sample_random_circuit::<_, 4, 11>(black_box(&mut circuit), &active_wires, &mut rng)
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
