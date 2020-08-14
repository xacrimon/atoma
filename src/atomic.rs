use crate::{ReclaimableManager, Reclaimer, Tag};
use std::{
    marker::PhantomData,
    sync::atomic::{AtomicUsize, Ordering},
};

pub struct Atomic<M: ReclaimableManager, R: Reclaimer<M>, T: Tag> {
    data: AtomicUsize,
    _m0: PhantomData<M>,
    _m1: PhantomData<R>,
    _m2: PhantomData<T>,
}
