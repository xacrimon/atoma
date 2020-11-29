use super::epoch::Epoch;
use crate::deferred::Deferred;
use tinyvec::ArrayVec;

const BAG_SIZE: usize = 32;

pub struct Bag {
    next: usize,
    deferred: ArrayVec<[Deferred; BAG_SIZE]>,
}

impl Bag {
    pub fn new() -> Self {
        Self {
            next: 0,
            deferred: ArrayVec::new(),
        }
    }

    pub fn push(&mut self, deferred: Deferred) {
        let next = self.next;
        self.next += 1;
        self.deferred[next] = deferred;
    }

    pub fn is_full(&self) -> bool {
        self.next == BAG_SIZE
    }

    pub fn seal(self, current_epoch: Epoch) -> SealedBag {
        SealedBag::new(current_epoch, self)
    }

    fn run(self) {
        for deferred in self.deferred {
            deferred.call();
        }
    }
}

pub struct SealedBag {
    epoch: Epoch,
    bag: Bag,
}

impl SealedBag {
    fn new(epoch: Epoch, bag: Bag) -> Self {
        Self { epoch, bag }
    }

    pub fn epoch(&self) -> Epoch {
        self.epoch
    }

    pub unsafe fn run(self) {
        self.bag.run();
    }
}
