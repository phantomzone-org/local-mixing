use criterion::{criterion_group, criterion_main, Criterion};
use local_mixing::{
    circuit::Circuit,
    local_mixing::{consts::N_OUT_KND, LocalMixingJob},
    replacement::strategy::{ControlFnChoice, ReplacementStrategy},
};
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

pub fn criterion_benchmark(c: &mut Criterion) {
    let mut rng = ChaCha8Rng::from_os_rng();

    for gates in [10_000, 100_000, 1_000_000, 10_000_000] {
        let circuit = Circuit::random(100, gates, &mut rng);
        let job = LocalMixingJob::new(
            100,
            0,
            100,
            1,
            10,
            ReplacementStrategy::Dummy,
            ControlFnChoice::All,
            circuit,
        );
        c.bench_function(&format!("search degredation gates={gates}"), |b| {
            b.iter_batched(
                || job.clone(),
                |mut job| job.execute_step::<_, N_OUT_KND>(&mut rng).unwrap(),
                criterion::BatchSize::PerIteration,
            );
        });
    }
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
