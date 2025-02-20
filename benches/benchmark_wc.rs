use criterion::{black_box, criterion_group, criterion_main, Criterion};
use local_mixing::{circuit::Circuit, replacement::is_weakly_connected};
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

pub fn criterion_benchmark(c: &mut Criterion) {
    let mut rng = ChaCha8Rng::from_os_rng();
    let circuit = Circuit::random(9, 4, &mut rng);

    c.bench_function("weakly connected", |b| {
        b.iter(|| black_box(is_weakly_connected(&circuit.gates)))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
