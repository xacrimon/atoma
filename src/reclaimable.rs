pub trait ReclaimableManager {
    type Reclaimable;

    fn reclaim(&self, object: &Self::Reclaimable);
}
