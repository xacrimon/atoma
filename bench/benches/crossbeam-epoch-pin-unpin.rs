use crossbeam_epoch::pin;
use bench::run_synced;
use std::sync::Arc;

fn main() {
    let ops_per_sec = run_synced(Arc::new(()), |_| drop(pin()));
    println!("crossbeam-epoch-pin-unpin: {} ops/sec", ops_per_sec);
}
