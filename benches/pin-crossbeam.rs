use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::thread;

const COUNT: usize = 1 << 23;

fn crossbeam_epoch() {
    let cpus = num_cpus::get();
    let mut handles = Vec::new();

    for _ in 0..cpus {
        handles.push(thread::spawn(move || {
            for _ in 0..COUNT {
                black_box(crossbeam_epoch::pin());
            }
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }
}

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("crossbeam-epoch 2^23", |b| b.iter(|| crossbeam_epoch()));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
