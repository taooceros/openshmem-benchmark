use std::sync::{Arc, atomic::AtomicBool};

use openshmem_benchmark::{
    osm_alloc::OsmMalloc,
    osm_box::OsmBox,
    osm_scope::{self, OsmScope},
    osm_vec::ShVec,
};
use openshmem_sys::_SHMEM_SYNC_VALUE;
use quanta::Instant;

use crate::operations::{Operation, OperationType};

pub fn run(operations: &Vec<Operation>, scope: &OsmScope) -> (usize, f64) {
    let mut false_signal = OsmBox::new(AtomicBool::new(false), &scope);
    let mut running = OsmBox::new(AtomicBool::new(true), &scope);

    let max_data_size = std::cmp::min(operations.iter().map(|e| e.size).max().unwrap(), 1024 * 1024 * 1024 * 64); // max 64GB

    let mut src = ShVec::<u8>::new(&scope);
    let mut dst = ShVec::<u8>::new(&scope);

    src.resize_with(max_data_size, || 0);
    dst.resize_with(max_data_size, || 0);

    scope.barrier_all();

    eprintln!("Running {} operations", operations.len());
    eprintln!("Max data size: {}", max_data_size);
    eprintln!("Number of PEs: {}", scope.num_pes());
    eprintln!("My PE: {}", scope.my_pe());

    let my_pe = scope.my_pe();
    let num_pes = scope.num_pes() / 2;

    let mut psync = ShVec::with_capacity(num_pes as usize, &scope);
    psync.resize_with(num_pes as usize, || _SHMEM_SYNC_VALUE as i64);

    let mut pwrk = ShVec::with_capacity(num_pes as usize, &scope);
    pwrk.resize_with(num_pes as usize, || 0);

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
                    num_ops += src.all_gather(
                        &mut dst,
                        scope,
                    );
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
                    num_ops += src.all_reduce(
                        &mut dst,
                        my_pe + num_pes as i32,
                        0,
                        num_pes as i32,
                        &mut pwrk,
                        &mut psync,
                        scope,
                    );
                }
                _ => {}
            }
        }
    } else {
        for operation in operations.iter() {

            eprintln!("Num ops: {} ({:?})", num_ops, operation.op_type);
            // periodically print the number of operations
            if num_ops - last_print_num_ops > 1000000 {
                let duration = Instant::now().duration_since(last_print_time);
                eprintln!("Num ops: {}", num_ops);
                eprintln!("Time: {:?}", duration);
                eprintln!("Op/s: {:0.2}", (num_ops - last_print_num_ops) as f64 / duration.as_secs_f64());
                last_print_time = Instant::now();;
                last_print_num_ops = num_ops;
                eprintln!("Num ops: {}", num_ops);
            }
            
            match operation.op_type {
                OperationType::Put => {
                    src[..operation.size].put_to_nbi(&mut dst, my_pe + num_pes as i32);
                    num_ops += 1;
                }
                OperationType::Get => {
                    src[..operation.size].get_from_nbi(&dst, my_pe + num_pes as i32);
                    num_ops += 1;
                }
                OperationType::PutNonBlocking => {
                    src[..operation.size].put_to_nbi(&mut dst, num_pes as i32);
                    num_ops += 1;
                }
                OperationType::GetNonBlocking => {
                    src[..operation.size].get_from_nbi(&dst, my_pe + num_pes as i32);
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
                    num_ops += src.all_gather(
                        &mut dst,
                        scope,
                    );
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
                    num_ops += src.all_reduce(
                        &mut dst,
                        scope,
                    );
                }
                OperationType::None => {
                    
                }
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
