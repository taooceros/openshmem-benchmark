use std::{
    mem::transmute,
    ops::{Deref, DerefMut},
};

use openshmem_sys::{shmem_putmem, shmem_putmem_nbi};

#[repr(transparent)]
pub struct OsmSlice<T> {
    data: [T],
}

impl<T> OsmSlice<T> {
    pub unsafe fn from_raw_parts<'a>(ptr: *mut T, len: usize) -> &'a Self {
        unsafe {
            let data = std::slice::from_raw_parts(ptr, len);
            transmute(data)
        }
    }

    pub unsafe fn from_raw_parts_mut<'a>(ptr: *mut T, len: usize) -> &'a mut Self {
        unsafe {
            let data = std::slice::from_raw_parts_mut(ptr, len);
            transmute(data)
        }
    }
}

impl<T> Deref for OsmSlice<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<T> DerefMut for OsmSlice<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

impl<T> OsmSlice<T> {
    pub fn put_to(&self, other: &mut Self, target_pe: i32) {
        unsafe {
            shmem_putmem(
                other.as_mut_ptr().cast(),
                self.as_ptr().cast(),
                std::mem::size_of::<T>() * self.len(),
                target_pe,
            );
        }
    }

    pub fn put_to_nbi(&self, other: &mut Self, target_pe: i32) {
        unsafe {
            shmem_putmem_nbi(
                other.as_mut_ptr().cast(),
                self.as_ptr().cast(),
                std::mem::size_of::<T>() * self.len(),
                target_pe,
            );
        }
    }

    pub fn get_from(&mut self, other: &Self, target_pe: i32) {
        unsafe {
            shmem_putmem(
                self.as_mut_ptr().cast(),
                other.as_ptr().cast(),
                std::mem::size_of::<T>() * self.len(),
                target_pe,
            );
        }
    }

    pub fn get_from_nbi(&mut self, other: &Self, target_pe: i32) {
        unsafe {
            shmem_putmem_nbi(
                self.as_mut_ptr().cast(),
                other.as_ptr().cast(),
                std::mem::size_of::<T>() * self.len(),
                target_pe,
            );
        }
    }
}
