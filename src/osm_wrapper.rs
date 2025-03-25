use std::ops::{Deref, DerefMut};

#[repr(transparent)]
pub struct OsmWrapper<T> {
    data: T,
}

impl<T> Deref for OsmWrapper<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<T> DerefMut for OsmWrapper<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}
