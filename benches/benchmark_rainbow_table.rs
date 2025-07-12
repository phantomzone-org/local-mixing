use criterion::{black_box, criterion_group, criterion_main, Criterion};
use local_mixing::{circuit::cf::GateLibrary, compression::ct::build_compression_table};

pub fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("compression-table", |b| {
        b.iter(|| {
            black_box(build_compression_table(2, 6, GateLibrary::NoIdentity));
        });
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
