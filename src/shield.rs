use super::{ReclaimableManager, Reclaimer};

pub struct Shield<'a, R, M>
where
    R: Reclaimer<M>,
    M: ReclaimableManager,
{
    reclaimer: &'a R,
    state: R::ShieldState,
}

impl<'a, R, M> Shield<'a, R, M>
where
    R: Reclaimer<M>,
    M: ReclaimableManager,
{
    pub fn retire(&self, parameter: R::RetireParameter) {
        self.reclaimer.retire(&self.state, parameter);
    }
}

impl<'a, R, M> Clone for Shield<'a, R, M>
where
    R: Reclaimer<M>,
    M: ReclaimableManager,
    R::ShieldState: Clone,
{
    fn clone(&self) -> Self {
        Self {
            reclaimer: self.reclaimer,
            state: self.state.clone(),
        }
    }
}

impl<'a, R, M> Drop for Shield<'a, R, M>
where
    R: Reclaimer<M>,
    M: ReclaimableManager,
{
    fn drop(&mut self) {
        self.reclaimer.drop_shield(&mut self.state);
    }
}
