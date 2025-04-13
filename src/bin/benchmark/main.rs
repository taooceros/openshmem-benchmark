#![feature(allocator_api)]

use crate::benchmark_loop::bandwidth_loop;
use std::iter::repeat_with;
use std::ops::Deref;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Instant;

use benchmark_loop::lantency_loop;
use bon::builder;
use clap::Parser;
use libc::gethostname;
use openshmem_benchmark::osm_box::OsmBox;
use openshmem_benchmark::osm_scope;
use openshmem_benchmark::osm_scope::OsmScope;
use openshmem_benchmark::osm_vec::ShVec;

use layout::RangeBenchmarkData;
use openshmem_sys::num_pes;
use ops::{
    AtomicOperation, BroadcastOperation, GetOperation, Operation, PutOperation, RangeOperation,
};

mod benchmark_loop;
mod layout;
mod ops;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Config {
    #[arg(global = true, long, default_value_t = 1)]
    epoch_size: usize,
    #[arg(global = true, short, long, default_value_t = 1)]
    size: usize,
    #[arg(global = true, short = 'n', long, default_value_t = 1000000)]
    epoch_per_iteration: usize,
    #[arg(global = true, short, long, value_enum)]
    duration: Option<u64>,
    #[arg(global = true, short = 'w', long, default_value_t = 4)]
    num_working_set: usize,
    #[command(subcommand)]
    operation: Operation,
    #[arg(global = true, short, long)]
    /// Measure Latency instead of Throughput
    /// Only valid for Blocking operations
    latency: bool,
}

fn main() {
    let config = Config::parse();
    benchmark(&config);
}

fn setup_exit_signal(timeout: Option<u64>, scope: &OsmScope) -> Arc<AtomicBool> {
    let local_running = Arc::new(std::sync::atomic::AtomicBool::new(true));
    let lr1 = local_running.clone();

    ctrlc::set_handler(move || {
        lr1.store(false, std::sync::atomic::Ordering::Relaxed);
    })
    .expect("Error setting Ctrl-C handler");

    if let Some(timeout) = timeout {
        if scope.my_pe() == 0 {
            let lr2 = local_running.clone();
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_secs(timeout));
                lr2.store(false, std::sync::atomic::Ordering::Relaxed);
            });
        }
    }

    return local_running;
}

fn print_config(config: &Config, scope: &OsmScope) {
    let pe = scope.my_pe();
    let num_pe = scope.num_pes();

    let hostname = unsafe {
        let mut name = [0; 256];
        let len = name.len();
        gethostname(name.as_mut_ptr() as *mut _, len);
        String::from_utf8_lossy(&name)
            .trim_end_matches('\0')
            .to_string()
    };

    // print config in format
    println!("Configuration on {}:{pe}:", hostname);
    println!("  Epoch Size: {}", config.epoch_size);
    println!("  Size: {}", config.size);
    println!("  Epoch per iteration: {}", config.epoch_per_iteration);
    println!("  Number of PEs: {}", num_pe);
    println!("  Duration: {:?}", config.duration);
    println!("  Operation: {}", config.operation);
    println!("  Number of Working Set: {}", config.num_working_set);
}

fn benchmark(cli: &Config) {
    let scope = osm_scope::OsmScope::init();

    print_config(cli, &scope);

    let local_running = setup_exit_signal(cli.duration, &scope);

    let mut running = OsmBox::new(AtomicBool::new(true), &scope);

    let operation = &cli.operation;
    let epoch_size = cli.epoch_size;
    let mut data_size = cli.size;

    // override data size for atomic operations
    match operation {
        Operation::Atomic { op: operation, .. } => match operation {
            AtomicOperation::FetchAdd32 => data_size = 4,
            AtomicOperation::FetchAdd64 => data_size = 8,
        },
        _ => {}
    }

    let num_pe = scope.num_pes();
    assert!(num_pe % 2 == 0, "Number of PEs must be even");
    let num_concurrency = (num_pe / 2) as usize;

    // When doing broadcast, let's try to use different memory locations for each PE
    let num_memory_location = match operation {
        Operation::Range(RangeOperation::Broadcast(_)) => num_concurrency,
        _ => 1,
    };

    let mut datas = repeat_with(|| {
        RangeBenchmarkData::setup_data()
            .data_size(data_size)
            .epoch_size(epoch_size)
            .scope(&scope)
            .num_working_set(cli.num_working_set)
            .call()
    })
    .take(num_memory_location)
    .collect::<Vec<_>>();
    let my_pe = scope.my_pe() as usize % num_concurrency;

    let data_id = match operation {
        Operation::Range(RangeOperation::Broadcast(_)) => my_pe,
        _ => 0,
    };

    let final_result = if cli.latency {
        lantency_loop()
            .scope(&scope)
            .local_running(local_running.clone())
            .running(&mut running)
            .operation(operation)
            .epoch_per_iteration(cli.epoch_per_iteration)
            .data(&mut datas[data_id])
            .call()
    } else {
        bandwidth_loop()
            .scope(&scope)
            .local_running(local_running.clone())
            .running(&mut running)
            .operation(operation)
            .epoch_per_iteration(cli.epoch_per_iteration)
            .data(&mut datas[data_id])
            .call()
    };

    output(&scope, num_concurrency, final_result, &cli);
}

fn output(scope: &OsmScope, num_concurrency: usize, final_result: f64, config: &Config) {
    // eprintln!("Final throughput: {:.2} messages/second", final_throughput);
    let my_pe = scope.my_pe() as usize;
    let op = &config.operation;

    let mut results = ShVec::with_capacity(num_concurrency * 2, &scope);

    results.resize_with(num_concurrency * 2, || 0.0);

    let my_throughput = OsmBox::new(final_result, &scope);

    match op {
        Operation::Range(RangeOperation::Get(_)) => {
            // only sync half the pe
            if my_pe >= num_concurrency {
                my_throughput.put_to_nbi(&mut results[my_pe], 0);
            }
        }
        Operation::Range(RangeOperation::Put(_))
        | Operation::Range(RangeOperation::Broadcast(_))
        | Operation::Range(RangeOperation::AllToAll)
        | Operation::Range(RangeOperation::PutGet { .. }) => {
            // only sync half the pe
            if my_pe < num_concurrency {
                my_throughput.put_to_nbi(&mut results[my_pe], 0);
            }
        }
        Operation::Atomic { .. } => {
            my_throughput.put_to_nbi(&mut results[my_pe], 0);
        }
    }

    // println!("pe {}: waiting for others", scope.my_pe());
    // sync all the pe
    scope.barrier_all();
    if scope.my_pe() == 0 {
        println!("Throughput on all PEs:");
        for i in 0..(num_concurrency * 2) {
            if *results[i] == 0.0 {
                continue;
            }
            eprintln!("PE {}: {:.2}", i, results[i].deref());
            println!(
                "PE {}: {:.2} Gbps",
                i,
                results[i].deref() * config.size as f64 / 1_000_000_000.0 * 8.0
            );
        }
    }
}
