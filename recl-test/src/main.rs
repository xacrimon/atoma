use flize::{Collector, Shield};
use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};

const THREADS: usize = 4;
const ALLOC_GROUP_SIZE: usize = 1024;
static ALLOCS: AtomicUsize = AtomicUsize::new(0);

fn main() {
    let collector = Arc::new(Collector::new());

    for _ in 0..THREADS {
        let collector = Arc::clone(&collector);

        thread::spawn(move || loop {
            for _ in 0..ALLOC_GROUP_SIZE {
                let a = Box::new(5_i32);
                let shield = collector.thin_shield();
                shield.retire(move || drop(a));
            }

            let alloc_groups = ALLOCS.fetch_add(1, Ordering::SeqCst) + 1;

            if alloc_groups % 100 == 0 {
                println!(
                    "completed {}x{} allocations",
                    alloc_groups, ALLOC_GROUP_SIZE
                );
            }
        });
    }

    loop {
        thread::sleep(Duration::from_secs(1));
    }
}
