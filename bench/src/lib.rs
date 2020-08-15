use std::{thread, sync::{Arc, Barrier}, time::Instant};

const TOTAL_OPS: usize = 1000000000;

pub fn run_synced<T: 'static + Send + Sync, F: 'static + FnMut(&T) + Clone + Send>(data: Arc<T>, f: F) -> f32 {
    let threads = num_cpus::get();
    let ops_per_thread = TOTAL_OPS / threads;
    let barrier = Arc::new(Barrier::new(threads + 1));

    for _ in 0..threads {
        let barrier = barrier.clone();
        let data = data.clone();
        let mut f = f.clone();

        thread::spawn(move || {
            barrier.wait();

            for _ in 0..ops_per_thread {
                f(&*data);
            }

            barrier.wait();
        });
    }

    barrier.wait();
    let start = Instant::now();

    barrier.wait();
    let micros = start.elapsed().as_micros() as f32;

    TOTAL_OPS as f32 / micros * 1000000 as f32
}
