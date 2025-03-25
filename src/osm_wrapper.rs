use std::{
    ffi::c_void, fmt::Display, ops::{Deref, DerefMut}
};

use openshmem_sys::{shmem_broadcastmem, shmem_putmem};

#[repr(transparent)]
pub struct OsmWrapper<T> {
    data: T,
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
    pub fn put_to(&self, target: &mut Self) {
        unsafe {
            shmem_putmem(
                target.deref_mut() as *mut T as *mut c_void,
                &self.data as *const T as *const c_void,
                std::mem::size_of::<T>(),
                0,
            );
        }
    }

    pub fn put_to_nbi(&self, target: &mut Self) {
        unsafe {
            shmem_putmem(
                target.deref_mut() as *mut T as *mut c_void,
                &self.data as *const T as *const c_void,
                std::mem::size_of::<T>(),
                0,
            );
        }
    }

    pub fn get_from(&mut self, source: &Self, size: usize) {
        unsafe {
            shmem_putmem(
                &mut self.data as *mut T as *mut c_void,
                source.deref() as *const T as *const c_void,
                size,
                0,
            );
        }
    }

    pub fn get_from_nbi(&mut self, source: &Self, size: usize) {
        unsafe {
            shmem_putmem(
                &mut self.data as *mut T as *mut c_void,
                source.deref() as *const T as *const c_void,
                size,
                0,
            );
        }
    }

    pub fn broadcast_to(
        &self,
        target: &mut Self,
        team: openshmem_sys::shmem_team_t,
        root_pe: i32,
    ) {
        unsafe {
            shmem_broadcastmem(
                team,
                target.deref_mut() as *mut T as *mut c_void,
                &self.data as *const T as *const c_void,
                std::mem::size_of::<T>(),
                root_pe,
            );
        }
    }
}
