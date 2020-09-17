use std::{
    sync::{Arc, Barrier},
    thread,
    time::{Duration, Instant},
};

const ITER: usize = 1 << 11;
const THREADS: usize = 8;

fn flize(collector: Arc<flize::Collector>) {
    let barrier = Arc::new(Barrier::new(THREADS + 1));

    for _ in 0..THREADS {
        let collector = Arc::clone(&collector);
        let barrier = Arc::clone(&barrier);

        thread::spawn(move || {
            barrier.wait();

            for _ in 0..ITER {
                let shield = collector.shield();

                for _ in 0..ITER {
                    shield.retire(|| ());
                }
            }

            barrier.wait();
        });
    }

    barrier.wait();
    barrier.wait();
}

fn crossbeam_epoch(collector: Arc<crossbeam_epoch::Collector>) {
    let barrier = Arc::new(Barrier::new(THREADS + 1));

    for _ in 0..THREADS {
        let collector = Arc::clone(&collector);
        let barrier = Arc::clone(&barrier);

        thread::spawn(move || {
            let handle = collector.register();
            barrier.wait();

            for _ in 0..ITER {
                let guard = handle.pin();

                for _ in 0..ITER {
                    guard.defer(|| ());
                }
            }

            barrier.wait();
        });
    }

    barrier.wait();
    barrier.wait();
}

fn main() {
    {
        let start = Instant::now();
        let collector = Arc::new(flize::Collector::new());
        let mut x = 0;

        while start.elapsed() < Duration::from_secs(30) {
            x += 1;
            flize(Arc::clone(&collector));
        }

        println!(
            "flize: called retire {} times in {} milliseconds",
            x * THREADS * ITER * ITER,
            start.elapsed().as_millis()
        );
    }

    {
        let start = Instant::now();
        let collector = Arc::new(crossbeam_epoch::Collector::new());
        let mut x = 0;

        while start.elapsed() < Duration::from_secs(30) {
            x += 1;
            crossbeam_epoch(Arc::clone(&collector));
        }

        println!(
            "crossbeam_epoch: called retire {} times in {} milliseconds",
            x * THREADS * ITER * ITER,
            start.elapsed().as_millis()
        );
    }
}
