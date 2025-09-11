use std::sync::{Arc, atomic::AtomicBool};

use openshmem_benchmark::{
    osm_alloc::OsmMalloc,
    osm_box::OsmBox,
    osm_scope::{self, OsmScope},
    osm_vec::ShVec,
};
use quanta::Instant;

use crate::operations::{Operation, OperationType};

pub fn run(operations: Vec<Operation>) {
    let scope = osm_scope::OsmScope::init();

    let mut false_signal = OsmBox::new(AtomicBool::new(false), &scope);
    let mut running = OsmBox::new(AtomicBool::new(true), &scope);

    let max_data_size = operations.iter().map(|e| e.size).max().unwrap();

    let mut src = ShVec::<u8>::new(&scope);
    let mut dst = ShVec::<u8>::new(&scope);

    src.resize_with(max_data_size, || 0);
    dst.resize_with(max_data_size, || 0);

    scope.barrier_all();

    eprintln!("Running {} operations", operations.len());
    eprintln!("Max data size: {}", max_data_size);
    eprintln!("Number of PEs: {}", scope.num_pes());
    eprintln!("My PE: {}", scope.my_pe());

    if scope.my_pe() == 1 {
        let mut counter = 0;
        while running.load(std::sync::atomic::Ordering::SeqCst) {
            scope.barrier_all();
        }
        return;
    }

    scope.barrier_all();

    let start = Instant::now();

    let mut barrier_counter = 0;

    for operation in operations.iter() {
        match operation.op_type {
            OperationType::Put => src[..operation.size].put_to(&mut dst, 1),
            OperationType::Get => src[..operation.size].get_from(&dst, 1),
            OperationType::PutNonBlocking => src[..operation.size].put_to_nbi(&mut dst, 1),
            OperationType::GetNonBlocking => src[..operation.size].get_from_nbi(&dst, 1),
            OperationType::Barrier => {
                scope.barrier_all();
            }
            OperationType::Fence => scope.fence(),
            OperationType::FetchAdd32 => {
                src.fetch_add_i32(1, 1);
            }
            OperationType::FetchAdd64 => {
                src.fetch_add_i64(1, 1);
            }
            OperationType::CompareAndSwap32 => {
                src.compare_and_swap_i32(1, 1, 1);
            }
            OperationType::CompareAndSwap64 => {
                src.compare_and_swap_i64(1, 1, 1);
            }
            _ => panic!("Unsupported operation"),
        }
    }

    scope.barrier_all();
    let end = Instant::now();

    false_signal.store(false, std::sync::atomic::Ordering::SeqCst);
    scope.barrier_all();
    false_signal.put_to_nbi(&mut running, 1);
    scope.barrier_all();

    println!("Time taken: {:?}", end.duration_since(start));
    println!(
        "Op/s: {:?}",
        operations.len() as f64 / end.duration_since(start).as_secs_f64()
    );
}
