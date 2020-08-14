use super::Reclaimer;

pub struct Shield<'a, R: Reclaimer> {
    reclaimer: &'a R,
    state: R::ShieldState,
}

impl<'a, R: Reclaimer> Shield<'a, R> {
    pub fn retire(&self, parameter: R::RetireParameter) {
        self.reclaimer.retire(&self.state, parameter);
    }
}

impl<'a, R: Reclaimer> Drop for Shield<'a, R> {
    fn drop(&mut self) {
        self.reclaimer.destroy_shield(&mut self.state);
    }
}
