use criterion::{black_box, criterion_group, criterion_main, Criterion};
use local_mixing::{
    circuit::Gate,
    replacement::{sample_random_circuit, strategy::ControlFnChoice},
};
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

pub fn criterion_benchmark(c: &mut Criterion) {
    let mut rng = ChaCha8Rng::from_os_rng();
    let mut circuit = [Gate::default(); 4];
    let active_wires = [
        [false, false, true, true, true, true, false, false, false],
        [true, true, true, true, true, true, false, false, false],
    ];
    let cf_choice = ControlFnChoice::OnlyUnique;

    c.bench_function("sample-circuit", |b| {
        b.iter(|| {
            black_box(sample_random_circuit(
                &mut circuit,
                &active_wires,
                cf_choice,
                &mut rng,
            ))
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
