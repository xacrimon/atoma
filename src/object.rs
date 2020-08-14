pub trait ObjectManager {
    type Object;

    fn reclaim(&self, object: &Self::Object);
}
