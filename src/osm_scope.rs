use openshmem_sys::*;

pub struct OsmScope;

impl OsmScope {
    pub fn init() -> Self {
        unsafe { shmem_init() };
        OsmScope
    }
}

impl Drop for OsmScope {
    fn drop(&mut self) {
        unsafe { shmem_finalize() };
    }
}

pub fn shmem_scope(f: impl FnOnce(OsmScope)) {
    let scope = OsmScope::init();
    f(scope);
}

impl OsmScope {
    pub fn my_pe(&self) -> i32 {
        unsafe { shmem_my_pe() }
    }

    pub fn barrier_all(&self) {
        unsafe { shmem_barrier_all() };
    }

    pub fn num_pes(&self) -> i32 {
        unsafe { shmem_n_pes() }
    }
}
