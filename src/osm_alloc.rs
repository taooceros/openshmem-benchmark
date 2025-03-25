use std::{
    alloc::{Allocator, GlobalAlloc}, backtrace::Backtrace, ptr::NonNull, sync::Arc
};

use openshmem_sys::{shfree, shmalloc, shmemalign, shrealloc};

use crate::osm_scope::OsmScope;

#[derive(Clone)]
pub struct OsmMalloc<'a> {
    _marker: std::marker::PhantomData<&'a ()>,
}

impl<'a> OsmMalloc<'a> {
    #[inline(always)]
    pub fn new(_scope: &'a OsmScope) -> Self {
        Self { _marker: std::marker::PhantomData }
    }
}

unsafe impl Allocator for OsmMalloc<'_> {
    fn allocate(
        &self,
        layout: std::alloc::Layout,
    ) -> Result<NonNull<[u8]>, std::alloc::AllocError> {
        let ptr = unsafe { shmemalign(layout.align(), layout.size()) };
        if ptr.is_null() {
            return Err(std::alloc::AllocError);
        }
        unsafe {
            Ok(NonNull::new_unchecked(std::ptr::slice_from_raw_parts_mut(
                ptr as *mut u8,
                layout.size(),
            )))
        }
    }

    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: std::alloc::Layout) {
        unsafe {
            shfree(ptr.as_ptr() as *mut std::ffi::c_void);
        }
    }

    unsafe fn grow(
        &self,
        ptr: NonNull<u8>,
        old_layout: std::alloc::Layout,
        new_layout: std::alloc::Layout,
    ) -> Result<NonNull<[u8]>, std::alloc::AllocError> {
        let new_ptr =
            unsafe { shrealloc(ptr.as_ptr() as *mut std::ffi::c_void, new_layout.size()) };
        if new_ptr.is_null() {
            return Err(std::alloc::AllocError);
        }
        unsafe {
            Ok(NonNull::new_unchecked(std::ptr::slice_from_raw_parts_mut(
                new_ptr as *mut u8,
                new_layout.size(),
            )))
        }
    }
}
