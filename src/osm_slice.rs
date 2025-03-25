use std::{
    mem::transmute,
    ops::{Deref, DerefMut, Index, IndexMut},
};

use openshmem_sys::{
    shmem_alltoallmem, shmem_broadcastmem, shmem_putmem, shmem_putmem_nbi, shmem_team_t,
};

use crate::osm_wrapper::OsmWrapper;

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

impl<T> Index<usize> for OsmSlice<T> {
    type Output = OsmWrapper<T>;

    fn index(&self, index: usize) -> &Self::Output {
        unsafe { transmute(&self.data[index]) }
    }
}

impl<T> IndexMut<usize> for OsmSlice<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        unsafe { transmute(&mut self.data[index]) }
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

    pub fn broadcast_to(&self, other: &mut Self, team: shmem_team_t, pe_root: i32) {
        unsafe {
            shmem_broadcastmem(
                team,
                other.as_mut_ptr().cast(),
                self.as_ptr().cast(),
                std::mem::size_of::<T>() * self.len(),
                pe_root,
            );
        }
    }
}
