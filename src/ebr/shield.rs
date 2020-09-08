use super::Collector;
use crate::deferred::Deferred;
use std::marker::PhantomData;

pub struct Shield<'collector> {
    collector: &'collector Collector,
    _m0: PhantomData<*mut ()>,
}

impl<'collector> Shield<'collector> {
    pub(crate) fn new(collector: &'collector Collector) -> Self {
        unsafe {
            collector.thread_state().enter(collector);
        }

        Self {
            collector,
            _m0: PhantomData,
        }
    }

    pub fn retire<F: FnOnce() + 'static>(&self, f: F) {
        let deferred = Deferred::new(f);
        self.collector.retire(deferred);
    }
}

impl<'collector> Drop for Shield<'collector> {
    fn drop(&mut self) {
        unsafe {
            self.collector.thread_state().exit(self.collector);
        }
    }
}
