use std::sync::atomic::{AtomicU8, Ordering};

const PIN_MASK: u8 = 0b10000000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Epoch {
    data: u8,
}

impl Epoch {
    pub const ZERO: Self = Self::from_raw(0);
    pub const ONE: Self = Self::from_raw(1);
    pub const TWO: Self = Self::from_raw(2);
    pub const THREE: Self = Self::from_raw(3);

    const fn from_raw(data: u8) -> Self {
        Self { data }
    }

    pub fn into_raw(self) -> u8 {
        self.data
    }

    pub fn is_pinned(self) -> bool {
        (self.data & PIN_MASK) != 0
    }

    pub fn pinned(self) -> Self {
        Self::from_raw(self.data | PIN_MASK)
    }

    pub fn unpinned(self) -> Self {
        Self::from_raw(self.data & !PIN_MASK)
    }

    pub fn next(self) -> Self {
        debug_assert!(!self.is_pinned());
        let data = (self.data + 1) % 4;
        Self::from_raw(data)
    }
}

pub struct AtomicEpoch {
    raw: AtomicU8,
}

impl AtomicEpoch {
    pub fn new(epoch: Epoch) -> Self {
        Self {
            raw: AtomicU8::new(epoch.into_raw()),
        }
    }

    pub fn load(&self, order: Ordering) -> Epoch {
        let raw = self.raw.load(order);
        Epoch::from_raw(raw)
    }
    pub fn store(&self, epoch: Epoch, order: Ordering) {
        let raw = epoch.into_raw();
        self.raw.store(raw, order);
    }

    pub fn swap_seq_cst(&self, new: Epoch) -> Epoch {
        let new_raw = new.into_raw();
        let previous_raw = self.raw.swap(new_raw, Ordering::SeqCst);
        Epoch::from_raw(previous_raw)
    }

    pub fn try_advance(&self, current: Epoch) -> Result<Epoch, ()> {
        let current_raw = current.into_raw();
        let next = current.next();
        let next_raw = next.into_raw();

        let did_advance = self.raw.compare_exchange_weak(
            current_raw,
            next_raw,
            Ordering::AcqRel,
            Ordering::AcqRel,
        );

        if did_advance.is_ok() {
            Ok(next)
        } else {
            Err(())
        }
    }
}
