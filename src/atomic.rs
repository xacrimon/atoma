use std::{marker::PhantomData, sync::atomic::{AtomicUsize, Ordering}};
use crate::{ReclaimableManager, Reclaimer};

pub struct Atomic<M: ReclaimableManager, R: Reclaimer<M>> {
    data: usize,
    _m0: PhantomData<M>,
    _m1: PhantomData<R>
}
