use criterion::{black_box, criterion_group, criterion_main, Criterion};
use local_mixing::compression::rainbow_table::populate_rainbow_table_brute_force;

pub fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("rainbow-table", |b| {
        b.iter(|| {
            black_box(populate_rainbow_table_brute_force::<1>());
        });
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
