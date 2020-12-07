use super::epoch::Epoch;
use crate::deferred::Deferred;
use tinyvec::ArrayVec;

pub struct Bag {
    deferred: ArrayVec<[(Deferred, Epoch); Self::SIZE]>,
}

impl Bag {
    pub const SIZE: usize = 32;

    pub fn new() -> Self {
        Self {
            deferred: ArrayVec::new(),
        }
    }

    pub fn push(&mut self, deferred: Deferred, epoch: Epoch) {
        self.deferred.push((deferred, epoch));
    }

    pub fn is_full(&self) -> bool {
        self.deferred.len() == Self::SIZE
    }

    pub fn is_empty(&self) -> bool {
        self.deferred.is_empty()
    }

    pub fn try_process(&mut self, current_epoch: Epoch) {
        while !self.deferred.is_empty() {
            let bottom_epoch = self.deferred[0].1;

            if bottom_epoch.two_passed(current_epoch) {
                self.deferred.remove(0).0.call();
            } else {
                break;
            }
        }
    }

    pub fn seal(self, current_epoch: Epoch) -> SealedBag {
        let data = self.deferred.into_iter().map(|(x, _)| x).collect();
        SealedBag::new(current_epoch, data)
    }
}

pub struct SealedBag {
    epoch: Epoch,
    deferred: ArrayVec<[Deferred; Bag::SIZE]>,
}

impl SealedBag {
    fn new(epoch: Epoch, deferred: ArrayVec<[Deferred; Bag::SIZE]>) -> Self {
        Self { epoch, deferred }
    }

    pub fn epoch(&self) -> Epoch {
        self.epoch
    }

    pub fn len(&self) -> usize {
        self.deferred.len()
    }

    pub unsafe fn run(self) -> usize {
        let mut x = 0;

        for deferred in self.deferred {
            deferred.call();
            x += 1;
        }

        x
    }
}
