use super::local::LocalState;
use crate::deferred::Deferred;

/// A `Shield` locks an epoch and is needed to manipulate protected atomic pointers.
/// It is a type level contract so that you are forces to acquire one before manipulating pointers.
/// This reduces common mistakes drastically since incorrect code will now fail at compile time.
pub struct Shield<'a> {
    local_state: &'a LocalState,
}

impl<'a> Shield<'a> {
    pub(crate) fn new(local_state: &'a LocalState) -> Shield<'a> {
        Self { local_state }
    }

    /// Attempt to synchronize the current thread to allow advancing the global epoch.
    /// This might be useful to call every once in a while if you plan on holding a `Shield`
    /// for an extended amount of time as to not stop garbage collection.
    ///
    /// This is only effective if this is the only active shield created by this thread.
    pub fn repin(&mut self) {
        unsafe {
            self.local_state.exit();
            self.local_state.enter();
        }
    }

    /// Attempt to synchronize the current thread like `Shield::repin` but executing a closure
    /// during the time the `Shield` is temporarily deactivated.
    pub fn repin_after<F, R>(&mut self, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        unsafe {
            self.local_state.exit();
            let value = f();
            self.local_state.enter();
            value
        }
    }

    /// Schedule a closure for execution once no shield may hold a reference
    /// to an object unlinked with the current shield.
    pub fn retire<F: FnOnce() + 'a>(&self, f: F) {
        let deferred = Deferred::new(f);
        self.local_state.retire(deferred, self);
    }
}

impl<'a> Clone for Shield<'a> {
    fn clone(&self) -> Self {
        unsafe {
            self.local_state.enter();
        }

        Self {
            local_state: self.local_state,
        }
    }
}

impl<'a> Drop for Shield<'a> {
    fn drop(&mut self) {
        unsafe {
            self.local_state.exit();
        }
    }
}

/// This is a utility type that allows you to either take a reference to a shield
/// and be bound by the lifetime of it or take an owned shield use `'static`.
#[derive(Clone)]
pub enum CowShield<'collector, 'shield> {
    Owned(Shield<'collector>),
    Borrowed(&'shield Shield<'collector>),
}

impl<'collector, 'shield> CowShield<'collector, 'shield> {
    pub fn new_owned(shield: Shield<'collector>) -> Self {
        CowShield::Owned(shield)
    }

    pub fn new_borrowed(shield: &'shield Shield<'collector>) -> Self {
        CowShield::Borrowed(shield)
    }

    pub fn into_owned(self) -> Shield<'collector> {
        match self {
            CowShield::Owned(shield) => shield,
            CowShield::Borrowed(shield) => shield.clone(),
        }
    }

    pub fn get(&self) -> &Shield<'collector> {
        match self {
            CowShield::Owned(shield) => shield,
            CowShield::Borrowed(shield) => shield,
        }
    }
}
