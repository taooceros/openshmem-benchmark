use std::ops::{Deref, DerefMut};

use crate::{osm_alloc::OsmMalloc, osm_slice::OsmSlice};

pub struct ShVec<T> {
    data: Vec<T, OsmMalloc>,
}

impl<T> ShVec<T> {
    pub fn new(size: usize) -> Self {
        let data = Vec::with_capacity_in(size, OsmMalloc);
        ShVec { data }
    }

    pub fn push(&mut self, value: T) {
        self.data.push(value);
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn with_capacity(size: usize) -> Self {
        let data = Vec::with_capacity_in(size, OsmMalloc);
        ShVec { data }
    }
}

impl<T> Deref for ShVec<T> {
    type Target = OsmSlice<T>;

    fn deref(&self) -> &Self::Target {
        unsafe {
            let ptr = self.data.as_ptr() as *mut T;
            let len = self.data.len();
            OsmSlice::from_raw_parts(ptr, len)
        }
    }
}

impl<T> DerefMut for ShVec<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe {
            let ptr = self.data.as_mut_ptr() as *mut T;
            let len = self.data.len();
            OsmSlice::from_raw_parts_mut(ptr, len)
        }
    }
}
