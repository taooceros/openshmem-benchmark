#![feature(allocator_api)]
use openshmem_sys::*;
use shalloc::ShMalloc;

mod shalloc;

fn main() {
    unsafe {
        shmem_init();

        let mut source = Vec::with_capacity_in(10, ShMalloc);
        let mut dest = Vec::<_, ShMalloc>::with_capacity_in(10, ShMalloc);
        dest.resize(10, 0);

        for i in 0..10 {
            source.push(i);
        }

        if shmem_my_pe() == 0 {
            println!("source: {:?}", source);
            shmem_long_put(dest.as_mut_ptr(), source.as_ptr(), source.len(), 1);
        }

        shmem_barrier_all();

        if shmem_my_pe() == 1 {
            println!("dest: {:?}", dest);
        }
    }

    unsafe {
        println!("Finalizing OpenSHMEM");
        shmem_finalize();
    }
}
