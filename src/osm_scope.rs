use openshmem_sys::*;

pub struct OsmScope;

impl OsmScope {
    pub fn init() -> Self {
        unsafe { shmem_init() };
        OsmScope
    }
}

static BARRIER_COUNT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
impl Drop for OsmScope {
    fn drop(&mut self) {
        let barrier_count = BARRIER_COUNT.load(std::sync::atomic::Ordering::Relaxed);
        println!("PE {}: barrier count {}", self.my_pe(), barrier_count);
        println!("Finalizing OpenSHMEM for pe {}", self.my_pe());

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
        BARRIER_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        unsafe { shmem_barrier_all() };
    }

    pub fn num_pes(&self) -> i32 {
        unsafe { shmem_n_pes() }
    }
}
