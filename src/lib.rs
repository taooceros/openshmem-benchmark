#![feature(allocator_api)]
use std::ffi::c_void;

use clap::Parser;
use openshmem_sys::*;
use osm_alloc::OsmMalloc;
use osm_vec::ShVec;

pub mod osm_alloc;
pub mod osm_box;
pub mod osm_slice;
pub mod osm_vec;
pub mod osm_wrapper;
pub mod osm_scope;

