use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::{sync::Arc, thread};

const COUNT: usize = 1 << 24;

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

fn flize() {
    let cpus = num_cpus::get();
    let collector = Arc::new(flize::Collector::new());
    let mut handles = Vec::new();

    for _ in 0..cpus {
        let collector = Arc::clone(&collector);

        handles.push(thread::spawn(move || {
            for _ in 0..COUNT {
                black_box(collector.shield());
            }
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }
}

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("crossbeam-epoch", |b| b.iter(|| crossbeam_epoch()));
    c.bench_function("flize", |b| b.iter(|| flize()));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
