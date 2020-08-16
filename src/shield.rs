use super::Reclaimer;

pub trait CloneShield<R>
where
    R: Reclaimer,
{
    fn clone_shield(&self, reclaimer: &R) -> Self;
}

pub struct Shield<'reclaimer, R>
where
    R: Reclaimer,
{
    reclaimer: &'reclaimer R,
    state: R::ShieldState,
}

impl<'reclaimer, R> Shield<'reclaimer, R>
where
    R: Reclaimer,
{
    pub(crate) fn new(reclaimer: &'reclaimer R, state: R::ShieldState) -> Self {
        Self { reclaimer, state }
    }

    pub fn retire(&self, parameter: R::Reclaimable) {
        self.reclaimer.retire(&self.state, parameter);
    }
}

impl<'reclaimer, R> Clone for Shield<'reclaimer, R>
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

impl<'reclaimer, R> Drop for Shield<'reclaimer, R>
where
    R: Reclaimer,
{
    fn drop(&mut self) {
        self.reclaimer.drop_shield(&mut self.state);
    }
}
