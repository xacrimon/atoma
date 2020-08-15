pub trait Reclaimer {
    type ShieldState;
    type Reclaimable;

    fn drop_shield(&self, state: &mut Self::ShieldState);
    fn retire(&self, state: &Self::ShieldState, param: Self::Reclaimable);
}

pub trait ReclaimableManager {
    type Reclaimable;

    fn reclaim(&self, object: Self::Reclaimable);
}
