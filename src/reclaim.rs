pub trait Reclaimer {
    type ShieldState;
    type RetireParameter;

    fn destroy_shield(&self, state: &mut Self::ShieldState);
    fn retire(&self, state: &Self::ShieldState, param: Self::RetireParameter);
}

pub trait ReclaimableManager {
    type Reclaimable;

    fn reclaim(&self, object: &Self::Reclaimable);
}
