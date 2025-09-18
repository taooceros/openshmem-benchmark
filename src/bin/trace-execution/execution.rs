use std::sync::{Arc, atomic::AtomicBool};

use openshmem_benchmark::{
    osm_alloc::OsmMalloc,
    osm_box::OsmBox,
    osm_scope::{self, OsmScope},
    osm_vec::ShVec,
};
use openshmem_sys::{_SHMEM_REDUCE_MIN_WRKDATA_SIZE, _SHMEM_REDUCE_SYNC_SIZE, _SHMEM_SYNC_VALUE};
use quanta::Instant;

use crate::operations::{Operation, OperationType};

pub fn run(operations: &Vec<Operation>, scope: &OsmScope) -> (usize, f64) {
    let mut false_signal = OsmBox::new(AtomicBool::new(false), &scope);
    let mut running = OsmBox::new(AtomicBool::new(true), &scope);

    let max_data_size = std::cmp::min(
        operations.iter().map(|e| e.size).max().unwrap(),
        1024 * 1024 * 1024 * 16,
    ); // max 16GB

    let max_reduce_type = operations
        .iter()
        .filter(|e| e.op_type == OperationType::AllReduce)
        .map(|e| e.size)
        .max()
        .unwrap();

    let mut src = ShVec::<u8>::new(&scope);
    let mut dst = ShVec::<u8>::new(&scope);

    let my_pe = scope.my_pe();
    let num_pes = scope.num_pes() / 2;

    src.resize_with(max_data_size, || 0);
    dst.resize_with(max_data_size * (num_pes as usize) * 2, || 0);

    scope.barrier_all();

    eprintln!("Running {} operations", operations.len());
    eprintln!("Max data size: {}", max_data_size);
    eprintln!("Number of PEs: {}", scope.num_pes());
    eprintln!("My PE: {}", scope.my_pe());

    let mut psync = ShVec::with_capacity(num_pes as usize, &scope);
    psync.resize_with(
        std::cmp::max(_SHMEM_REDUCE_SYNC_SIZE as usize, num_pes as usize),
        || _SHMEM_SYNC_VALUE as i64,
    );

    let mut pwrk = ShVec::with_capacity(num_pes as usize, &scope);
    pwrk.resize_with(
        std::cmp::max(_SHMEM_REDUCE_MIN_WRKDATA_SIZE as usize, num_pes as usize),
        || 0i32,
    );

    scope.barrier_all();

    let start = Instant::now();
    let mut last_print_time = Instant::now();
    let mut last_print_num_ops = 0;
    let mut num_ops = 0;

    if scope.my_pe() >= num_pes {
        let mut counter = 0;
        for operation in operations.iter() {
            match operation.op_type {
                // OperationType::Barrier => scope.barrier_all(),
                OperationType::AllGather => {
                    num_ops += src.all_gather(&mut dst, scope, &mut psync);
                }
                OperationType::AllToAll => {
                    num_ops += src.all_to_all(
                        &mut dst,
                        my_pe + num_pes as i32,
                        0,
                        num_pes as i32,
                        &mut psync,
                        scope,
                    );
                }
                OperationType::AllReduce => {
                    num_ops += src.all_reduce(&mut dst, scope, &mut pwrk, &mut psync);
                }
                _ => {}
            }
        }
    } else {
        for operation in operations.iter() {
            let cnt = std::cmp::min(operation.size, max_data_size);
            // periodically print the number of operations
            if num_ops - last_print_num_ops > 2 {
                let duration = Instant::now().duration_since(last_print_time);
                eprintln!("Num ops: {}", num_ops);
                eprintln!("Time: {:?}", duration);
                eprintln!(
                    "Op/s: {:0.2}",
                    (num_ops - last_print_num_ops) as f64 / duration.as_secs_f64()
                );
                last_print_time = Instant::now();
                last_print_num_ops = num_ops;
                eprintln!("Num ops: {}", num_ops);
            }

            match operation.op_type {
                OperationType::Put => {
                    src[..cnt].put_to_nbi(&mut dst, my_pe + num_pes as i32);
                    num_ops += 1;
                }
                OperationType::Get => {
                    src[..cnt].get_from_nbi(&dst, my_pe + num_pes as i32);
                    num_ops += 1;
                }
                OperationType::PutNonBlocking => {
                    src[..cnt].put_to_nbi(&mut dst, num_pes as i32);
                    num_ops += 1;
                }
                OperationType::GetNonBlocking => {
                    src[..cnt].get_from_nbi(&dst, my_pe + num_pes as i32);
                    num_ops += 1;
                }
                OperationType::Barrier => {
                    // scope.barrier_all();
                }
                OperationType::Fence => scope.fence(),
                OperationType::FetchAdd32 => {
                    src.fetch_add_i32(1, my_pe + num_pes as i32);
                }
                OperationType::FetchAdd64 => {
                    src.fetch_add_i64(1, my_pe + num_pes as i32);
                }
                OperationType::CompareAndSwap32 => {
                    src.compare_and_swap_i32(1, 1, my_pe + num_pes as i32);
                }
                OperationType::CompareAndSwap64 => {
                    src.compare_and_swap_i64(1, 1, my_pe + num_pes as i32);
                }
                OperationType::AllGather => {
                    num_ops += src[..cnt].all_gather(&mut dst, scope, &mut psync);
                }
                OperationType::AllToAll => {
                    num_ops += src.all_to_all(
                        &mut dst,
                        my_pe + num_pes as i32,
                        0,
                        num_pes as i32,
                        &mut psync,
                        scope,
                    );
                }
                OperationType::AllReduce => {
                    num_ops += src[..cnt].all_reduce(&mut dst, scope, &mut pwrk, &mut psync);
                }
                OperationType::None => {}
                _ => panic!("Unsupported operation"),
            }
        }
    }

    scope.barrier_all();
    let end = Instant::now();

    false_signal.store(false, std::sync::atomic::Ordering::SeqCst);
    scope.barrier_all();
    false_signal.put_to_nbi(&mut running, 1);
    scope.barrier_all();

    return (num_ops, end.duration_since(start).as_secs_f64());
}
