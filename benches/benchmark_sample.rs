use criterion::{black_box, criterion_group, criterion_main, Criterion};
use local_mixing::{
    circuit::Gate,
    replacement::{sample_circuit_lookup, sample_random_circuit, strategy::ControlFnChoice},
};
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

pub fn criterion_benchmark(c: &mut Criterion) {
    let mut rng = ChaCha8Rng::from_os_rng();
    let mut circuit = [Gate::default(); 4];
    let active_wires = [
        [true, false, false, true, false, false, false, false, false],
        [false, true, false, false, true, false, false, false, false],
    ];
    let cf_choice = ControlFnChoice::OnlyUnique;

    c.bench_function("sample circuit guided", |b| {
        b.iter(|| {
            black_box(sample_random_circuit(
                &mut circuit,
                &active_wires,
                cf_choice,
                &mut rng,
            ))
        })
    });

    c.bench_function("sample circuit lookup", |b| {
        b.iter(|| {
            black_box(sample_circuit_lookup(
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
