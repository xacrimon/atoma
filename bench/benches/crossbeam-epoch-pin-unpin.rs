use criterion::{criterion_group, criterion_main, Criterion};
use crossbeam_epoch::pin;

fn bench_pin(c: &mut Criterion) {
    c.bench_function("crossbeam-epoch pin & unpin", |b| b.iter(|| pin()));
}

criterion_group!(benches, bench_pin);
criterion_main!(benches);
