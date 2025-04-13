use core::str;
use std::mem::{MaybeUninit, transmute};

use libc::DS;
use openshmem_sys::{oshmem_team_world, shmem_broadcast64, shmem_team_split_strided};
use openshmem_sys::{shmem_broadcastmem, shmem_team_t};

use crate::osm_slice::OsmSlice;

#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
pub struct OsmTeam {
    pub inner: shmem_team_t,
}

impl OsmTeam {
    pub fn world() -> Self {
        unsafe { OsmTeam { inner: oshmem_team_world } }
    }

    pub fn split_strided(
        self,
        start: i32,
        stride: i32,
        size: i32,
    ) -> Result<Self, TeamCreationError> {
        let mut team = MaybeUninit::uninit();
        unsafe {
            let result = shmem_team_split_strided(
                self.inner,
                start,
                stride,
                size,
                std::ptr::null_mut(),
                0,
                team.as_mut_ptr(),
            );

            if result != 0 {
                return Err(TeamCreationError::Fail);
            }

            Ok(OsmTeam {
                inner: team.assume_init(),
            })
        }
    }

    pub fn broadcast<T>(self, src: &OsmSlice<T>, dst: &mut OsmSlice<T>, pe_root: i32) {
        unsafe {
            shmem_broadcastmem(
                self.inner,
                dst.as_mut_ptr().cast(),
                src.as_ptr().cast(),
                std::mem::size_of::<T>() * src.len(),
                pe_root,
            );
        }
    }
}

pub enum TeamCreationError {
    Fail,
}
