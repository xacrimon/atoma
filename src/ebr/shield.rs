use super::local::LocalState;
use crate::deferred::Deferred;

pub struct Shield<'a> {
    local_state: &'a LocalState,
}

impl<'a> Shield<'a> {
    pub(crate) fn new(local_state: &'a LocalState) -> Shield<'a> {
        Self { local_state }
    }

    pub fn repin(&mut self) {
        unsafe {
            self.local_state.exit();
            self.local_state.enter();
        }
    }

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
