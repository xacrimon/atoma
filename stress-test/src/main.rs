use flize::Collector;
use std::{
    sync::{Arc, Barrier},
    thread,
    time::{Duration, Instant},
};

const ITER: usize = 1 << 11;
const THREADS: usize = 8;

fn reclaim_multi_thread(collector: Arc<Collector>) {
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

fn main() {
    let start = Instant::now();
    let collector = Arc::new(Collector::new());
    let mut x = 0;

    while start.elapsed() < Duration::from_secs(20) {
        x += 1;
        reclaim_multi_thread(Arc::clone(&collector));
    }

    println!("called retire {} times in {} milliseconds", x * THREADS * ITER, start.elapsed().as_millis());
}
