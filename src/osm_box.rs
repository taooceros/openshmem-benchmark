use crate::osm_alloc::OsmMalloc;

pub struct OsmBox<T> {
    data: Box<T, OsmMalloc>,
}

impl<T> OsmBox<T> {
    pub fn new(data: T) -> Self {
        let data = Box::new_in(data, OsmMalloc);
        OsmBox { data }
    }
}

impl<T> std::ops::Deref for OsmBox<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}
