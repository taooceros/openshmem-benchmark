use std::{mem::transmute, ops::Deref, sync::Arc};

use crate::{osm_alloc::OsmMalloc, osm_scope::OsmScope, osm_wrapper::OsmWrapper};

pub struct OsmArc<'a, T> {
    data: Arc<T, OsmMalloc<'a>>,
}

impl<'a, T> OsmArc<'a, T> {
    pub fn new(data: T, scope: &'a OsmScope) -> Self {
        let data = Arc::new_in(data, OsmMalloc::new(scope));
        OsmArc { data }
    }
}

impl<T> Deref for OsmArc<'_, T> {
    type Target = OsmWrapper<T>;

    fn deref(&self) -> &Self::Target {
        unsafe {
            transmute(&self.data)
        }
    }
}
