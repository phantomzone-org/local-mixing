use criterion::{black_box, criterion_group, criterion_main, Criterion};
use local_mixing::{
    circuit::Circuit,
    local_mixing::{consts::N_OUT_KND, LocalMixingJob},
    replacement::strategy::{ControlFnChoice, ReplacementStrategy},
};
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

pub fn criterion_benchmark(c: &mut Criterion) {
    let mut rng = ChaCha8Rng::from_os_rng();
    let circuit = Circuit::load_from_binary("degredation.bin").unwrap();
    let job = LocalMixingJob::new(100, 0, 100, 1, 10, ReplacementStrategy::Dummy, ControlFnChoice::All, circuit);

    c.bench_function("search degredation", |b| {
        b.iter(|| {
            let mut job = job.clone();
            black_box(job.execute_step::<_, N_OUT_KND>(&mut rng));
        });
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
