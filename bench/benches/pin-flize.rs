use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::{sync::Arc, thread};

const COUNT: usize = 1 << 23;

fn flize() {
    let cpus = num_cpus::get();
    let collector = Arc::new(flize::Collector::new());
    let mut handles = Vec::new();

    for _ in 0..cpus {
        let collector = Arc::clone(&collector);

        handles.push(thread::spawn(move || {
            let local = collector.local();

            for _ in 0..COUNT {
                black_box(local.thin_shield());
            }
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }
}

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("flize", |b| b.iter(|| flize()));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
