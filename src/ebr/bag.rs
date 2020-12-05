use super::epoch::Epoch;
use crate::deferred::Deferred;
use tinyvec::ArrayVec;

pub struct Bag {
    next: usize,
    deferred: ArrayVec<[Deferred; Self::SIZE]>,
}

impl Bag {
    pub const SIZE: usize = 32;

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
        self.next == Self::SIZE
    }

    pub fn is_empty(&self) -> bool {
        self.next == 0
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
