use std::mem::transmute;

use crate::{osm_alloc::OsmMalloc, osm_scope::OsmScope, osm_wrapper::OsmWrapper};

pub struct OsmBox<'a, T> {
    data: Box<T, OsmMalloc<'a>>,
}

impl<'a, T> OsmBox<'a, T> {
    pub fn new(data: T, scope: &'a OsmScope) -> Self {
        let allocator = OsmMalloc::new(scope);
        let data = Box::new_in(data, allocator);
        OsmBox { data }
    }
}

impl<'a, T> std::ops::Deref for OsmBox<'a, T> {
    type Target = OsmWrapper<T>;

    fn deref(&self) -> &Self::Target {
        unsafe {
            transmute(self.data.deref())
        }
    }
}

impl<'a, T> std::ops::DerefMut for OsmBox<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe {
            transmute(self.data.deref_mut())
        }
    }
}