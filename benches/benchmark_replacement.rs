use criterion::{black_box, criterion_group, criterion_main, Criterion};
use local_mixing::{
    circuit::{cf::GateLibrary, Gate},
    replacement::find_replacement_random_sample,
};
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

pub fn criterion_benchmark(c: &mut Criterion) {
    let mut rng = ChaCha8Rng::from_os_rng();
    let circuit = [
        Gate {
            wires: [0, 1, 2],
            control_func: 3,
            generation: 0,
        },
        Gate {
            wires: [1, 3, 4],
            control_func: 9,
            generation: 0,
        },
    ];

    c.bench_function("replacement", |b| {
        b.iter(|| {
            black_box(find_replacement_random_sample(
                &circuit,
                9,
                4,
                1_000_000_000,
                GateLibrary::OnlyUnique,
                true,
                &mut rng,
            ))
        });
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
