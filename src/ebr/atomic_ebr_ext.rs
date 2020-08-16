use crate::{Atomic, Reclaimer, Shared, Shield, Tag};
use std::sync::atomic::Ordering;

pub trait AtomicEbrExt {
    type R: Reclaimer;
    type V;
    type T: Tag;

    fn load<'shield>(
        &self,
        order: Ordering,
        _shield: &'shield Shield<'_, Self::R>,
    ) -> Shared<'shield, Self::V, Self::T>;

    fn store<'shield>(&self, new: Shared<'_, Self::V, Self::T>, order: Ordering);

    fn swap<'shield>(
        &self,
        new: Shared<'_, Self::V, Self::T>,
        order: Ordering,
        _shield: &'shield Shield<'_, Self::R>,
    ) -> Shared<'shield, Self::V, Self::T>;

    fn compare_and_swap<'shield>(
        &self,
        current: Shared<'_, Self::V, Self::T>,
        new: Shared<'_, Self::V, Self::T>,
        order: Ordering,
        _shield: &'shield Shield<'_, Self::R>,
    ) -> Shared<'shield, Self::V, Self::T>;
}

impl<R, V, T> AtomicEbrExt for Atomic<R, V, T>
where
    R: Reclaimer,
    T: Tag,
{
    type R = R;
    type V = V;
    type T = T;

    fn load<'shield>(
        &self,
        order: Ordering,
        _shield: &'shield Shield<'_, Self::R>,
    ) -> Shared<'shield, Self::V, Self::T> {
        let data = self.data.load(order);
        Shared::from_raw(data)
    }

    fn store<'shield>(&self, new: Shared<'_, Self::V, Self::T>, order: Ordering) {
        let data = new.into_raw();
        self.data.store(data, order);
    }

    fn swap<'shield>(
        &self,
        new: Shared<'_, Self::V, Self::T>,
        order: Ordering,
        _shield: &'shield Shield<'_, Self::R>,
    ) -> Shared<'shield, Self::V, Self::T> {
        let new_data = new.into_raw();
        let previous_data = self.data.swap(new_data, order);
        Shared::from_raw(previous_data)
    }

    fn compare_and_swap<'shield>(
        &self,
        current: Shared<'_, Self::V, Self::T>,
        new: Shared<'_, Self::V, Self::T>,
        order: Ordering,
        _shield: &'shield Shield<'_, Self::R>,
    ) -> Shared<'shield, Self::V, Self::T> {
        let current_data = current.into_raw();
        let new_data = new.into_raw();
        let previous_data = self.data.compare_and_swap(current_data, new_data, order);
        Shared::from_raw(previous_data)
    }
}
