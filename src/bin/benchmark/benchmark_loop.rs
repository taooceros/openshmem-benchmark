use std::{
    mem::transmute,
    ops::Deref,
    sync::{Arc, atomic::AtomicBool},
    time::Instant,
};

use bon::builder;
use openshmem_benchmark::{osm_box::OsmBox, osm_scope};

use crate::{
    RangeBenchmarkData,
    ops::{self, AtomicOperation, GetOperation, Operation, PutOperation, RangeOperation},
};

#[builder]
pub fn benchmark_loop<'a>(
    scope: &osm_scope::OsmScope,
    local_running: Arc<AtomicBool>,
    running: &mut OsmBox<'a, AtomicBool>,
    operation: Operation,
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

            let begin = Instant::now();
            for (src, dst) in source.iter_mut().zip(dest.iter_mut()) {
                match operation {
                    // TODO: add validation for range operation
                    Operation::RangeOperation(RangeOperation::Put(operation)) => {
                        if my_pe < num_concurrency {
                            match operation {
                                PutOperation::Put => {
                                    src.put_to(dst, (my_pe + num_concurrency) as i32);
                                }
                                PutOperation::PutNonBlocking => {
                                    src.put_to_nbi(dst, (my_pe + num_concurrency) as i32);
                                }
                            }
                        }
                    }
                    Operation::RangeOperation(RangeOperation::Get(operation)) => {
                        if my_pe >= num_concurrency {
                            match operation {
                                GetOperation::Get => {
                                    src.get_from(dst, (my_pe + num_concurrency) as i32);
                                }
                                GetOperation::GetNonBlocking => {
                                    src.get_from_nbi(dst, (my_pe + num_concurrency) as i32);
                                }
                            }
                        }
                    }

                    Operation::RangeOperation(RangeOperation::Broadcast) => {
                        if my_pe < num_concurrency {
                            src.broadcast_to(dst, num_concurrency..(num_concurrency * 2));
                        }
                    }
                    Operation::Atomic { op: operation, use_different_location } => {
                        let target_pe = if !use_different_location { 0 } else { my_pe % num_concurrency };

                        match operation {
                            AtomicOperation::FetchAdd32 => {
                                dst.fetch_add_i32(seed as i32, target_pe as i32);
                            }
                            AtomicOperation::FetchAdd64 => {
                                dst.fetch_add_i64(seed as i64, target_pe as i32);
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

            let now = Instant::now();
            // scope.barrier_all();
            if epoch % 1000 == 0 {
                // println!("pe {my_pe} {epoch} barrier elapsed time: {}", now.elapsed().as_micros());
            }

            // let now = std::time::Instant::now();

            if my_pe >= num_concurrency {
                if let Operation::RangeOperation(_) = operation {
                    let check_epoch = seed % epoch_size;
                    let check_data = seed % data_size;
                    unsafe {
                        if source.get_unchecked(check_epoch).get_unchecked(check_data)
                            != dest.get_unchecked(check_epoch).get_unchecked(check_data)
                        {
                            println!(
                                "pe {my_pe} epoch {epoch} check failed: {:?} != {:?}",
                                source[check_epoch], dest[check_epoch]
                            );
                        }
                    }
                }
            }
        }

        let elapsed = now.elapsed();

        if final_throughput == 0.0 || running.load(std::sync::atomic::Ordering::SeqCst) {
            let total_messages = epoch_per_iteration * epoch_size;
            let throughput = total_messages as f64 / elapsed.as_secs_f64();
            // println!(
            //     "Throughput on Machine {my_pe}: {:.2} messages/second",
            //     throughput
            // );

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
