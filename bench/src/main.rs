use std::{thread, sync::{Arc, Barrier}, time::Instant};
use flize::{ebr::Ebr, function_runner::FunctionRunner};

const TOTAL_OPS: usize = 5000000000;

fn run_synced<T: 'static + Send + Sync, F: 'static + FnMut(&T) + Clone + Send>(data: Arc<T>, f: F) -> f32 {
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
    let micros = start.elapsed().as_nanos() as f32;

    TOTAL_OPS as f32 / micros * 1000000000 as f32
}

fn main() {
    let ops_per_sec = run_synced(Arc::new(()), |_| drop(crossbeam_epoch::pin()));
    println!("crossbeam-epoch-pin-unpin: {} ops/sec", ops_per_sec);

    let reclaimer = Arc::new(Ebr::new(FunctionRunner));
    let ops_per_sec = run_synced(reclaimer, |reclaimer| drop(reclaimer.shield()));
    println!("flize-epoch-pin-unpin: {} ops/sec", ops_per_sec);
}
