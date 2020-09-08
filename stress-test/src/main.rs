use flize::Collector;
use std::{sync::{Arc, Barrier}, thread};

const ITER: usize = 1 << 12;

fn reclaim_single_thread() {
    let collector = Collector::new();

    for _ in 0..ITER {
        let shield = collector.shield();
        
        for _ in 0..ITER {
            shield.retire(|| ());
        }
    }
}

fn reclaim_multi_thread() {
    let collector = Arc::new(Collector::new());
    let barrier = Arc::new(Barrier::new(9));

    for _ in 0..8 {
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
    reclaim_single_thread();
    reclaim_multi_thread();
}
