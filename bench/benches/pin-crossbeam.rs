use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::{sync::Arc, thread};

const COUNT: usize = 1 << 23;

fn crossbeam_epoch() {
    let cpus = num_cpus::get();
    let collector = Arc::new(crossbeam_epoch::Collector::new());
    let mut handles = Vec::new();

    for _ in 0..cpus {
        let collector = Arc::clone(&collector);

        handles.push(thread::spawn(move || {
            let local = collector.register();

            for _ in 0..COUNT {
                black_box(local.pin());
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
