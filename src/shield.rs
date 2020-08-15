use super::Reclaimer;

pub trait CloneShield<R>
where
    R: Reclaimer,
{
    fn clone_shield(&self, reclaimer: &R) -> Self;
}

pub struct Shield<'a, R>
where
    R: Reclaimer,
{
    reclaimer: &'a R,
    state: R::ShieldState,
}

impl<'a, R> Shield<'a, R>
where
    R: Reclaimer,
{
    pub(crate) fn new(reclaimer: &'a R, state: R::ShieldState) -> Self {
        Self { reclaimer, state }
    }

    pub fn retire(&self, parameter: R::Reclaimable) {
        self.reclaimer.retire(&self.state, parameter);
    }
}

impl<'a, R> Clone for Shield<'a, R>
where
    R: Reclaimer,
    R::ShieldState: CloneShield<R>,
{
    fn clone(&self) -> Self {
        let state = self.state.clone_shield(self.reclaimer);

        Self {
            reclaimer: self.reclaimer,
            state,
        }
    }
}

impl<'a, R> Drop for Shield<'a, R>
where
    R: Reclaimer,
{
    fn drop(&mut self) {
        self.reclaimer.drop_shield(&mut self.state);
    }
}
