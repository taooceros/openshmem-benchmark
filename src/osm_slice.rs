use std::{
    mem::transmute,
    ops::{Deref, DerefMut, Index, IndexMut},
};

use openshmem_sys::{
    shmem_barrier, shmem_broadcast64, shmem_broadcastmem, shmem_getmem, shmem_getmem_nbi, shmem_int_atomic_fetch_add, shmem_int_broadcast, shmem_int_fadd, shmem_long_atomic_fetch_add, shmem_putmem, shmem_putmem_nbi, shmem_team_t, SHMEM_BARRIER_SYNC_SIZE, _SHMEM_SYNC_VALUE
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
            shmem_getmem(
                self.as_mut_ptr().cast(),
                other.as_ptr().cast(),
                std::mem::size_of::<T>() * self.len(),
                target_pe,
            );
        }
    }

    pub fn get_from_nbi(&mut self, other: &Self, target_pe: i32) {
        unsafe {
            shmem_getmem_nbi(
                self.as_mut_ptr().cast(),
                other.as_ptr().cast(),
                std::mem::size_of::<T>() * self.len(),
                target_pe,
            );
        }
    }

    pub fn broadcast(
        &self,
        other: &mut Self,
        root_pe: i32,
        pe_start: i32,
        log_pe_stride: i32,
        pe_size: i32,
    ) {
        unsafe {
            let mut pSync = vec![_SHMEM_SYNC_VALUE as i64; SHMEM_BARRIER_SYNC_SIZE as usize * pe_size as usize];

            shmem_broadcast64(
                other.as_mut_ptr().cast(),
                self.as_ptr().cast(),
                self.len() * std::mem::size_of::<T>() / std::mem::size_of::<u64>(),
                root_pe,
                pe_start,
                log_pe_stride,
                pe_size,
                pSync.as_mut_ptr(),
            );
        }
    }

    pub fn fetch_add_i32(&mut self, value: i32, target_pe: i32) -> i32 {
        unsafe {
            if self.len() != size_of::<i32>() {
                panic!("fetch_add_i32 only works for i32");
            }

            shmem_int_atomic_fetch_add(self.as_mut_ptr().cast(), value, target_pe)
        }
    }

    pub fn fetch_add_i64(&mut self, value: i64, target_pe: i32) -> i64 {
        unsafe {
            if self.len() != size_of::<i64>() {
                panic!("fetch_add_i64 only works for i64");
            }

            shmem_long_atomic_fetch_add(self.as_mut_ptr().cast(), value, target_pe)
        }
    }
}
