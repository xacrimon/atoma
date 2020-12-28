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
        let collect_until = self
            .deferred
            .iter()
            .filter(|(_, epoch)| epoch.two_passed(current_epoch))
            .fuse()
            .count();

        self.deferred
            .drain(..collect_until)
            .for_each(|(deferred, _)| deferred.call());
    }

    fn last_epoch(&self) -> Epoch {
        let len = self.deferred.len();

        if len != 0 {
            unsafe { self.deferred.get_unchecked(len - 1).1 }
        } else {
            Epoch::ZERO
        }
    }

    pub fn seal(self) -> SealedBag {
        let epoch = self.last_epoch();
        let data = self.deferred.into_iter().map(|(x, _)| x).collect();
        SealedBag::new(epoch, data)
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
        let x = self.deferred.len();

        for deferred in self.deferred {
            deferred.call();
        }

        x
    }
}
