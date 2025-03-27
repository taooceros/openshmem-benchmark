use std::ops::{Deref, DerefMut};

use crate::{osm_alloc::OsmMalloc, osm_scope::OsmScope, osm_slice::OsmSlice};

#[derive(Debug)]
pub struct ShVec<'a, T> {
    data: Vec<T, OsmMalloc<'a>>,
}

impl<'a, T> ShVec<'a, T> {
    pub fn new(scope: &'a OsmScope) -> Self {
        let data = Vec::new_in(OsmMalloc::new(scope));
        ShVec { data }
    }

    pub fn push(&mut self, value: T) {
        self.data.push(value);
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn with_capacity(size: usize, scope: &'a OsmScope) -> Self {
        let data = Vec::with_capacity_in(size, OsmMalloc::new(scope));
        ShVec { data }
    }

    pub fn resize_with(&mut self, size: usize, f: impl Fn() -> T) {
        self.data.resize_with(size, f);
    }
}

impl<'a, T> Deref for ShVec<'a, T> {
    type Target = OsmSlice<T>;

    fn deref(&self) -> &Self::Target {
        unsafe {
            let ptr = self.data.as_ptr() as *mut T;
            let len = self.data.len();
            OsmSlice::from_raw_parts(ptr, len)
        }
    }
}

impl<'a, T> DerefMut for ShVec<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe {
            let ptr = self.data.as_mut_ptr() as *mut T;
            let len = self.data.len();
            OsmSlice::from_raw_parts_mut(ptr, len)
        }
    }
}
