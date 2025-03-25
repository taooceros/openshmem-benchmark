use crate::{osm_alloc::OsmMalloc, osm_scope::OsmScope};

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
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}
