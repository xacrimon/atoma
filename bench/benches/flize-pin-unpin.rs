use flize::{ebr::Ebr, function_runner::FunctionRunner};
use bench::run_synced;
use std::sync::Arc;

fn main() {
    let reclaimer = Arc::new(Ebr::new(FunctionRunner));
    let ops_per_sec = run_synced(reclaimer, |reclaimer| drop(reclaimer.shield()));
    println!("flize-epoch-pin-unpin: {} ops/sec", ops_per_sec);
}
