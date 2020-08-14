use super::{Reclaimer, ReclaimableManager};

pub struct Shield<'a, M: ReclaimableManager, R: Reclaimer<M>> {
    reclaimer: &'a R,
    state: R::ShieldState,
}

impl<'a, M: ReclaimableManager, R: Reclaimer<M>> Shield<'a, M, R> {
    pub fn retire(&self, parameter: R::RetireParameter) {
        self.reclaimer.retire(&self.state, parameter);
    }
}

impl<'a, M: ReclaimableManager, R: Reclaimer<M>> Clone for Shield<'a, M, R>
where
    R::ShieldState: Clone,
{
    fn clone(&self) -> Self {
        Self {
            reclaimer: self.reclaimer,
            state: self.state.clone(),
        }
    }
}

impl<'a, M: ReclaimableManager, R: Reclaimer<M>> Drop for Shield<'a, M, R> {
    fn drop(&mut self) {
        self.reclaimer.drop_shield(&mut self.state);
    }
}
