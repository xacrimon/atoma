use std::sync::atomic::{AtomicUsize, Ordering};

const PIN_MASK: usize = 0b1000_0000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Epoch {
    data: usize,
}

impl Epoch {
    pub const AMOUNT: usize = 3;
    pub const ZERO: Self = Self::from_raw(0);

    #[inline]
    const fn from_raw(data: usize) -> Self {
        Self { data }
    }

    #[inline]
    pub fn into_raw(self) -> usize {
        self.data
    }

    #[inline]
    pub fn is_pinned(self) -> bool {
        (self.data & PIN_MASK) != 0
    }

    #[inline]
    pub fn pinned(self) -> Self {
        Self::from_raw(self.data | PIN_MASK)
    }

    #[inline]
    pub fn unpinned(self) -> Self {
        Self::from_raw(self.data & !PIN_MASK)
    }

    #[inline]
    pub fn next(self) -> Self {
        debug_assert!(!self.is_pinned());
        let data = (self.data + 1) % Self::AMOUNT;
        Self::from_raw(data)
    }
}

pub struct AtomicEpoch {
    raw: AtomicUsize,
}

impl AtomicEpoch {
    #[inline]
    pub fn new(epoch: Epoch) -> Self {
        Self {
            raw: AtomicUsize::new(epoch.into_raw()),
        }
    }

    #[inline]
    pub fn load(&self, order: Ordering) -> Epoch {
        let raw = self.raw.load(order);
        Epoch::from_raw(raw)
    }

    #[inline]
    pub fn store(&self, epoch: Epoch, order: Ordering) {
        let raw = epoch.into_raw();
        self.raw.store(raw, order);
    }

    #[cold]
    #[inline(never)]
    pub fn try_advance(&self, current: Epoch) -> Result<Epoch, ()> {
        let current_raw = current.into_raw();
        let next = current.next();
        let next_raw = next.into_raw();

        let did_advance = self.raw.compare_exchange_weak(
            current_raw,
            next_raw,
            Ordering::AcqRel,
            Ordering::Relaxed,
        );

        if did_advance.is_ok() {
            Ok(next)
        } else {
            Err(())
        }
    }
}
