use std::sync::atomic::{AtomicUsize, Ordering};

pub struct Rcu {
    readers: AtomicUsize,
}

impl Rcu {
    pub fn new() -> Self {
        Self {
            readers: AtomicUsize::new(0),
        }
    }

    pub fn read_lock(&self) {
        self.readers.fetch_add(1, Ordering::SeqCst);
    }

    pub fn read_unlock(&self) {
        self.readers.fetch_sub(1, Ordering::SeqCst);
    }

    pub fn synchronize(&self) {
        while self.readers.load(Ordering::SeqCst) != 0 {}
    }
}
