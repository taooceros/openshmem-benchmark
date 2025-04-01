use std::{
    ffi::c_void,
    fmt::Display,
    ops::{Deref, DerefMut},
};

use openshmem_sys::shmem_putmem;

#[repr(transparent)]
pub struct OsmWrapper<T> {
    data: T,
}

impl<T: PartialEq> PartialEq for OsmWrapper<T> {
    fn eq(&self, other: &Self) -> bool {
        self.data == other.data
    }
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

impl<T: Display> Display for OsmWrapper<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.data)
    }
}

impl<T> OsmWrapper<T> {
    pub fn put_to(&self, target: &mut Self, pe: i32) {
        unsafe {
            shmem_putmem(
                target.deref_mut() as *mut T as *mut c_void,
                &self.data as *const T as *const c_void,
                std::mem::size_of::<T>(),
                pe,
            );
        }
    }

    pub fn put_to_nbi(&self, target: &mut Self, pe: i32) {
        unsafe {
            shmem_putmem(
                target.deref_mut() as *mut T as *mut c_void,
                &self.data as *const T as *const c_void,
                std::mem::size_of::<T>(),
                pe,
            );
        }
    }

    pub fn get_from(&mut self, source: &Self, size: usize, pe: i32) {
        unsafe {
            shmem_putmem(
                &mut self.data as *mut T as *mut c_void,
                source.deref() as *const T as *const c_void,
                size,
                pe,
            );
        }
    }

    pub fn get_from_nbi(&mut self, source: &Self, size: usize, pe: i32) {
        unsafe {
            shmem_putmem(
                &mut self.data as *mut T as *mut c_void,
                source.deref() as *const T as *const c_void,
                size,
                pe,
            );
        }
    }
}
