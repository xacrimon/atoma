use core::sync::atomic::{AtomicU64, Ordering};

const PIN_MASK: u64 = core::u64::MAX >> 1;

#[derive(Default, Debug, Clone, Copy)]
pub struct Epoch {
    data: u64,
}

impl Epoch {
    pub const AMOUNT: u64 = 3;
    pub const ZERO: Self = Self::from_raw(0);

    const fn from_raw(data: u64) -> Self {
        Self { data }
    }

    pub fn into_raw(self) -> u64 {
        self.data
    }

    pub fn is_pinned(self) -> bool {
        (self.data & !PIN_MASK) != 0
    }

    pub fn pinned(self) -> Self {
        Self::from_raw(self.data | !PIN_MASK)
    }

    pub fn unpinned(self) -> Self {
        Self::from_raw(self.data & PIN_MASK)
    }

    pub fn next(self) -> Self {
        debug_assert!(!self.is_pinned());
        Self::from_raw(self.data + 1)
    }

    fn unique(self) -> u64 {
        self.data % Self::AMOUNT
    }
}

impl PartialEq for Epoch {
    fn eq(&self, other: &Self) -> bool {
        self.unique() == other.unique()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DefinitiveEpoch(pub u64);

impl From<Epoch> for DefinitiveEpoch {
    fn from(epoch: Epoch) -> Self {
        Self(epoch.data)
    }
}

pub struct AtomicEpoch {
    raw: AtomicU64,
}

impl AtomicEpoch {
    pub fn new(epoch: Epoch) -> Self {
        Self {
            raw: AtomicU64::new(epoch.into_raw()),
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

    pub fn compare_and_set_non_unique(&self, current: Epoch, new: Epoch, order: Ordering) {
        let current_raw = current.into_raw();
        let new_raw = new.into_raw();
        self.raw.compare_and_swap(current_raw, new_raw, order);
    }

    pub fn try_advance(&self, current: Epoch) -> Result<Epoch, ()> {
        let current_raw = current.into_raw();
        let next = current.next();
        let next_raw = next.into_raw();

        let did_advance =
            self.raw
                .compare_exchange(current_raw, next_raw, Ordering::SeqCst, Ordering::Relaxed);

        if did_advance.is_ok() {
            Ok(next)
        } else {
            Err(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Epoch;

    #[test]
    fn pin_check() {
        let epoch = Epoch::ZERO;
        assert!(!epoch.is_pinned());
        let pinned = epoch.pinned();
        assert!(pinned.is_pinned());
        let unpinned_next = epoch.unpinned().next();
        assert!(!unpinned_next.is_pinned());
    }
}
