use core::panic;
use std::{
    arch::x86_64::__rdtscp,
    mem::transmute,
    ops::Deref,
    path::{Path, PathBuf},
    sync::{Arc, atomic::AtomicBool},
    time::{Duration, Instant},
};

use bon::builder;
use openshmem_benchmark::{osm_box::OsmBox, osm_scope, osm_team::OsmTeam, osm_vec::ShVec};
use openshmem_sys::_SHMEM_SYNC_VALUE;

use crate::{
    RangeBenchmarkData,
    ops::{
        self, AtomicOperation, BroadcastOperation, GetOperation, Operation, PutGetOp,
        PutGetOperation, PutOperation, RangeOperation,
    },
};

#[builder]
pub fn lantency_loop<'a>(
    scope: &osm_scope::OsmScope,
    local_running: Arc<AtomicBool>,
    running: &mut OsmBox<'a, AtomicBool>,
    operation: &Operation,
    epoch_per_iteration: usize,
    data: &mut RangeBenchmarkData<'a>,
) -> f64 {
    let mut final_latency = 0.0;

    let my_pe = scope.my_pe() as usize;
    let num_pe = scope.num_pes() as usize;
    let num_concurrency = (num_pe / 2) as usize;

    let epoch_size = data.epoch_size();
    let data_size = data.data_size();
    const PRIME: usize = 1_000_000_007;
    let mut seed = 0;
    let num_working_set = data.num_working_set();
    let false_signal = OsmBox::new(AtomicBool::new(false), &scope);

    let mut aux = 0;

    loop {
        scope.barrier_all();
        let mut cycles = Vec::with_capacity(epoch_per_iteration);

        if !running.load(std::sync::atomic::Ordering::SeqCst) {
            break;
        }

        let now = Instant::now();

        for _ in 0..(epoch_per_iteration) {
            seed = (1 + seed * 7) % PRIME;

            let i = seed % num_working_set;

            let source = &mut data.src_working_set[i][0];

            let dest = &mut data.dst_working_set[i][0];

            let begin_cycle = unsafe { __rdtscp(&mut aux) };

            match operation {
                Operation::Range(RangeOperation::Get(GetOperation::Get)) => {
                    if my_pe >= num_concurrency {
                        dest.get_from(source, (my_pe - num_concurrency) as i32);
                    }
                }
                Operation::Range(RangeOperation::Put(PutOperation::Put)) => {
                    if my_pe < num_concurrency {
                        source.put_to(dest, (my_pe + num_concurrency) as i32);
                    }
                }
                Operation::Range(RangeOperation::Broadcast(BroadcastOperation::Broadcast)) => {
                    source.broadcast(dest, 0, 0, 0, num_pe as i32);
                }
                Operation::Atomic {
                    op,
                    use_different_location,
                } => {
                    let target_pe = if *use_different_location {
                        (my_pe % num_concurrency) as i32
                    } else {
                        0
                    };
                    match op {
                        AtomicOperation::FetchAdd32 => {
                            dest.fetch_add_i32(1, target_pe);
                        }
                        AtomicOperation::FetchAdd64 => {
                            dest.fetch_add_i64(1, target_pe);
                        }
                        AtomicOperation::CompareAndSwap32 => {
                            dest.compare_and_swap_i32(0, 0, target_pe);
                        }
                        AtomicOperation::CompareAndSwap64 => {
                            dest.compare_and_swap_i64(0, 0, target_pe);
                        }
                    }
                }
                _ => unreachable!("This operation should not be here. {operation:?}"),
            }

            let end_cycle = unsafe { __rdtscp(&mut aux) };

            cycles.push(end_cycle - begin_cycle);
        }

        match operation {
            Operation::Range(RangeOperation::Get(GetOperation::Get)) => {
                if my_pe >= num_concurrency {
                    record_latency(
                        running,
                        epoch_per_iteration,
                        &mut final_latency,
                        &cycles,
                        now,
                        my_pe,
                    );
                }
            }
            Operation::Range(RangeOperation::Put(PutOperation::Put)) => {
                if my_pe < num_concurrency {
                    record_latency(
                        running,
                        epoch_per_iteration,
                        &mut final_latency,
                        &cycles,
                        now,
                        my_pe,
                    );
                }
            }
            Operation::Range(RangeOperation::Broadcast(BroadcastOperation::Broadcast)) => {
                if my_pe == 0 {
                    record_latency(
                        running,
                        epoch_per_iteration,
                        &mut final_latency,
                        &cycles,
                        now,
                        my_pe,
                    );
                }
            }
            Operation::Atomic { .. } => {
                record_latency(
                    running,
                    epoch_per_iteration,
                    &mut final_latency,
                    &cycles,
                    now,
                    my_pe,
                );
            }

            _ => unreachable!("This operation {operation} should not be here."),
        }

        // let only the main pe to stop others
        if !local_running.load(std::sync::atomic::Ordering::Relaxed) && my_pe == 0 {
            // set the running flag to false
            for i in 0..num_pe as i32 {
                println!("pe {}: stopping pe {}", scope.my_pe(), i);
                false_signal.put_to_nbi(running, i);
            }
        }
    }

    final_latency
}

fn record_latency<'a>(
    running: &mut OsmBox<'a, AtomicBool>,
    epoch_per_iteration: usize,
    final_latency: &mut f64,
    latency_cycles: &Vec<u64>,
    now: Instant,
    my_pe: usize,
) {
    let latency = now.elapsed();

    if *final_latency == 0.0 || running.load(std::sync::atomic::Ordering::Relaxed) {
        println!(
            "Latency on Machine {my_pe}: {:.2} microseconds",
            latency.as_nanos() as f64 / epoch_per_iteration as f64 / 1000.0
        );

        // efficiently print min/median/mean/max cycle
        let mut cycles = latency_cycles.clone();
        cycles.sort();
        let min_latency = cycles[0];
        let median_latency = if cycles.len() % 2 == 0 {
            (cycles[cycles.len() / 2] + cycles[cycles.len() / 2 - 1]) as f64 / 2.0
        } else {
            cycles[cycles.len() / 2] as f64
        };
        let mean_latency = cycles.iter().sum::<u64>() as f64 / cycles.len() as f64;
        let max_latency = cycles[cycles.len() - 1];

        println!(
            "Latency on Machine {my_pe}: min: {:.2} median: {:.2} mean: {:.2} max: {:.2}",
            min_latency, median_latency, mean_latency, max_latency
        );

        *final_latency = latency.as_nanos() as f64 / epoch_per_iteration as f64 / 1000.0;
    }
}

#[builder]
pub fn bandwidth_loop<'a>(
    scope: &osm_scope::OsmScope,
    local_running: Arc<AtomicBool>,
    running: &mut OsmBox<'a, AtomicBool>,
    operation: &Operation,
    epoch_per_iteration: usize,
    data: &mut RangeBenchmarkData<'a>,
) -> f64 {
    let mut final_throughput = 0.0;
    let my_pe = scope.my_pe() as usize;
    let num_pe = scope.num_pes() as usize;
    let num_concurrency = (num_pe / 2) as usize;

    let epoch_size = data.epoch_size();
    let data_size = data.data_size();
    const PRIME: usize = 1_000_000_007;
    let mut seed = 0;
    let num_working_set = data.num_working_set();

    if let Operation::Atomic { op: operation, .. } = operation {
        if ![4usize, 8usize].contains(&data_size) {
            panic!("Atomic operation requires data size to be 4 or 8 bytes (int or long)");
        }
    }

    let mut op_seq = None;

    let mut put_get_op_seq = match operation {
        Operation::Range(RangeOperation::PutGet {
            op_sequence,
            op_sequence_file,
            ..
        }) => op_sequence.as_ref().map_or_else(
            || {
                if let Some(op_sequence_file) = op_sequence_file {
                    op_seq = Some(ops::read_op_sequence(Path::new(op_sequence_file)));
                    let op_seq = op_seq.as_ref().unwrap();
                    if op_seq.is_empty() {
                        panic!("Empty operation sequence file");
                    }
                    Some(op_seq.iter().cycle())
                } else {
                    return None;
                }
            },
            |op_seq| Some(op_seq.iter().cycle()),
        ),
        _ => None,
    };

    let mut all_to_all_dst = if let Operation::Range(RangeOperation::AllToAll) = operation {
        let mut all_to_all_dst = ShVec::with_capacity(data_size * num_pe, scope);
        all_to_all_dst.resize_with(data_size * num_pe, || 0);
        Some(all_to_all_dst)
    } else {
        None
    };

    let mut psync = ShVec::with_capacity(num_pe, scope);
    psync.resize_with(num_pe, || _SHMEM_SYNC_VALUE as i64);

    let false_signal = OsmBox::new(AtomicBool::new(false), &scope);

    loop {
        scope.barrier_all();

        if !running.load(std::sync::atomic::Ordering::SeqCst) {
            break;
        }

        let now = Instant::now();

        for epoch in 0..(epoch_per_iteration) {
            seed = (1 + seed * 7) % PRIME;
            let i = seed % num_working_set;

            let source = &mut data.src_working_set[i];
            let dest = &mut data.dst_working_set[i];

            // let begin = Instant::now();
            for (src, dst) in source.iter_mut().zip(dest.iter_mut()) {
                match operation {
                    // TODO: add validation for range operation
                    Operation::Range(RangeOperation::Put(operation)) => {
                        if my_pe < num_concurrency {
                            let target_pe = my_pe + num_concurrency;
                            match operation {
                                PutOperation::Put => {
                                    src.put_to(dst, target_pe as i32);
                                }
                                PutOperation::PutNonBlocking => {
                                    src.put_to_nbi(dst, target_pe as i32);
                                }
                            }
                        }
                    }
                    Operation::Range(RangeOperation::Get(operation)) => {
                        if my_pe >= num_concurrency {
                            let target_pe = my_pe - num_concurrency;
                            match operation {
                                GetOperation::Get => {
                                    src.get_from(dst, target_pe as i32);
                                }
                                GetOperation::GetNonBlocking => {
                                    src.get_from_nbi(dst, target_pe as i32);
                                }
                            }
                        }
                    }
                    Operation::Range(RangeOperation::PutGet {
                        blocking,
                        put_ratio,
                        ..
                    }) => {
                        if my_pe < num_concurrency {
                            if let Some(put_ratio) = put_ratio {
                                let target_pe = my_pe + num_concurrency;
                                if seed % 100 < (put_ratio * 100.0) as usize {
                                    if *blocking {
                                        src.put_to(dst, target_pe as i32);
                                    } else {
                                        src.put_to_nbi(dst, target_pe as i32);
                                    }
                                } else {
                                    if *blocking {
                                        src.get_from(dst, target_pe as i32);
                                    } else {
                                        src.get_from_nbi(dst, target_pe as i32);
                                    }
                                }
                            } else if let Some(op_seq) = put_get_op_seq.as_mut() {
                                match op_seq.next().as_ref().unwrap() {
                                    PutGetOp::Put => {
                                        if *blocking {
                                            src.put_to(dst, (my_pe + num_concurrency) as i32);
                                        } else {
                                            src.put_to_nbi(dst, (my_pe + num_concurrency) as i32);
                                        }
                                    }
                                    PutGetOp::Get => {
                                        if *blocking {
                                            src.get_from(dst, (my_pe + num_concurrency) as i32);
                                        } else {
                                            src.get_from_nbi(dst, (my_pe + num_concurrency) as i32);
                                        }
                                    }
                                }
                            } else {
                                panic!(
                                    "PutGet operation requires either put_ratio or op_sequence to be set"
                                );
                            }
                        }
                    }

                    Operation::Range(RangeOperation::Broadcast(operation)) => match operation {
                        BroadcastOperation::Broadcast => {
                            src.broadcast(dst, 0, 0, 0, num_pe as i32);
                        }
                    },

                    Operation::Range(RangeOperation::AllToAll) => {
                        src.all_to_all(
                            all_to_all_dst.as_mut().expect(
                                "All to All dest should be initilized when testing all to all.",
                            ),
                            0,
                            0,
                            num_pe as i32,
                            &mut psync,
                        );
                    }
                    Operation::Atomic {
                        op: operation,
                        use_different_location,
                    } => {
                        let target_pe = if !use_different_location {
                            0
                        } else {
                            my_pe % num_concurrency
                        };

                        match operation {
                            AtomicOperation::FetchAdd32 => {
                                dst.fetch_add_i32(seed as i32, target_pe as i32);
                            }
                            AtomicOperation::FetchAdd64 => {
                                dst.fetch_add_i64(seed as i64, target_pe as i32);
                            }
                            AtomicOperation::CompareAndSwap32 => {
                                dst.compare_and_swap_i32(2, 0, target_pe as i32);
                            }
                            AtomicOperation::CompareAndSwap64 => {
                                dst.compare_and_swap_i64(2, 0, target_pe as i32);
                            }
                        }
                    }
                };
            }

            if epoch % 1000 == 0 {
                // println!(
                //     "pe {my_pe} {epoch} elapsed time: {}",
                //     begin.elapsed().as_micros()
                // );
            }

            // let now = Instant::now();
            scope.barrier_all();
            if epoch % 1000 == 0 {
                // println!("pe {my_pe} {epoch} barrier elapsed time: {}", now.elapsed().as_micros());
            }

            // let now = std::time::Instant::now();

            // if my_pe >= num_concurrency {
            //     if let Operation::Range(_) = operation {
            //         let check_epoch = seed % epoch_size;
            //         let check_data = seed % data_size;
            //         unsafe {
            //             if source.get_unchecked(check_epoch).get_unchecked(check_data)
            //                 != dest.get_unchecked(check_epoch).get_unchecked(check_data)
            //             {
            //                 println!(
            //                     "pe {my_pe} epoch {epoch} check failed: {:?} != {:?}",
            //                     source[check_epoch], dest[check_epoch]
            //                 );
            //             }
            //         }
            //     }
            // }
        }

        let elapsed = now.elapsed();

        let total_messages = epoch_per_iteration * epoch_size;
        let throughput = total_messages as f64 / elapsed.as_secs_f64();
        if my_pe < num_concurrency {
            println!(
                "Throughput on Machine {my_pe}: {:.2} messages/second",
                throughput
            );
        }

        if let Operation::Range(RangeOperation::Broadcast(_)) = operation {
            if my_pe != 0 {
                // skip other pes when testing broadcast
                continue;
            }
        }

        if final_throughput == 0.0 || running.load(std::sync::atomic::Ordering::Relaxed) {
            final_throughput = throughput;
        }

        // let only the main pe to stop others
        if !local_running.load(std::sync::atomic::Ordering::Relaxed) && my_pe == 0 {
            // set the running flag to false
            for i in 0..num_pe as i32 {
                println!("pe {}: stopping pe {}", scope.my_pe(), i);
                false_signal.put_to_nbi(running, i);
            }
        }
    }

    final_throughput
}
