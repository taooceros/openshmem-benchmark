#![feature(allocator_api)]

use std::ops::Deref;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use bon::builder;
use clap::Parser;
use openshmem_benchmark::osm_alloc::OsmMalloc;
use openshmem_benchmark::osm_arc::OsmArc;
use openshmem_benchmark::osm_scope;
use openshmem_benchmark::osm_vec::ShVec;
use openshmem_sys::{my_pe, shmem_char_p};

#[derive(clap::ValueEnum, Debug, Clone, Copy)]
enum Operation {
    Put,
    Get,
}

impl ToString for Operation {
    fn to_string(&self) -> String {
        match self {
            Operation::Put => "put".to_string(),
            Operation::Get => "get".to_string(),
        }
    }
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Config {
    #[arg(long, default_value_t = 1)]
    epoch_size: usize,
    #[arg(short, long, default_value_t = 1)]
    size: usize,
    #[arg(short = 'n', long, default_value_t = 1000000)]
    epoch_per_iteration: usize,
    #[arg(short = 'p', long, default_value_t = 1)]
    num_pe: usize,
    #[arg(short, long, value_enum)]
    duration: Option<u64>,
    #[arg(short, long, default_value_t = Operation::Put)]
    operation: Operation,
}

fn main() {
    let config = Config::parse();

    // print config in format
    println!("Configuration:");
    println!("  Epoch Size: {}", config.epoch_size);
    println!("  Size: {}", config.size);
    println!("  Epoch per iteration: {}", config.epoch_per_iteration);
    println!("  Number of PEs: {}", config.num_pe);
    println!("  Duration: {:?}", config.duration);
    println!("  Operation: {}", config.operation.to_string());

    benchmark(&config);
}

fn setup_exit_signal(timeout: Option<u64>) -> Arc<AtomicBool> {
    let local_running = Arc::new(std::sync::atomic::AtomicBool::new(true));
    let lr1 = local_running.clone();

    ctrlc::set_handler(move || {
        lr1.store(false, std::sync::atomic::Ordering::Relaxed);
    })
    .expect("Error setting Ctrl-C handler");

    if let Some(timeout) = timeout {
        let lr2 = local_running.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_secs(timeout));
            lr2.store(false, std::sync::atomic::Ordering::Relaxed);
        });
    }

    return local_running;
}

#[builder]
fn setup_data<'a>(
    scope: &'a osm_scope::OsmScope,
    epoch_size: usize,
    data_size: usize,
    num_pe: usize,
) -> (Vec<ShVec<'a, u8>>, Vec<ShVec<'a, u8>>) {
    let mut source = Vec::with_capacity(epoch_size);
    let mut dest = Vec::with_capacity(epoch_size);

    for i in 0..epoch_size {
        let mut source_entry = ShVec::with_capacity(data_size, scope);
        let mut dest_entry = ShVec::with_capacity(data_size, scope);
        for j in 0..data_size {
            source_entry.push((i * data_size + j) as u8);
        }
        dest_entry.resize_with(data_size, || 0);
        source.push(source_entry);
        dest.push(dest_entry);
    }

    (source, dest)
}

fn benchmark(cli: &Config) {
    let scope = osm_scope::OsmScope::init();

    let local_running = setup_exit_signal(if scope.my_pe() == 0 {
        cli.duration
    } else {
        None
    });

    let running = OsmArc::new(AtomicBool::new(true), &scope);

    let operation = cli.operation;
    let epoch_size = cli.epoch_size;
    let data_size = cli.size;
    let num_pe = cli.num_pe;

    let mut sources = Vec::with_capacity(num_pe);
    let mut dests = Vec::with_capacity(num_pe);

    for _ in 0..num_pe {
        let (source, dest) = setup_data()
            .data_size(data_size)
            .epoch_size(epoch_size)
            .num_pe(cli.num_pe)
            .scope(&scope)
            .call();

        sources.push(source);
        dests.push(dest);
    }

    let my_pe = scope.my_pe() as usize % num_pe;
    let target_pe = my_pe;

    let final_throughput = benchmark_loop()
        .scope(&scope)
        .local_running(local_running.clone())
        .running(&running)
        .operation(operation)
        .epoch_per_iteration(cli.epoch_per_iteration)
        .epoch_size(epoch_size)
        .num_pe(cli.num_pe)
        .source(&mut sources[my_pe])
        .dest(&mut dests[target_pe])
        .call();

    println!("pe {}: stopping benchmark", scope.my_pe());

    // let only the main pe to stop others
    if scope.my_pe() == 0 {
        unsafe {
            shmem_char_p(running.as_ptr() as *mut i8, false as i8, 1);
        }
        scope.barrier_all();
    }
    eprintln!("Final throughput: {:.2} messages/second", final_throughput);

    println!("Finalizing OpenSHMEM for pe {}", scope.my_pe());
}

#[builder]
fn benchmark_loop<'a>(
    scope: &osm_scope::OsmScope,
    local_running: Arc<AtomicBool>,
    running: &OsmArc<'a, AtomicBool>,
    operation: Operation,
    epoch_per_iteration: usize,
    epoch_size: usize,
    num_pe: usize,
    source: &mut Vec<ShVec<'a, u8>>,
    dest: &mut Vec<ShVec<'a, u8>>,
) -> f64 {
    let mut final_throughput = 0.0;
    let my_pe = scope.my_pe() as usize;

    while running.load(std::sync::atomic::Ordering::Relaxed)
        && local_running.load(std::sync::atomic::Ordering::Relaxed)
    {
        let now = std::time::Instant::now();
        for _ in 0..epoch_per_iteration {
            if my_pe < num_pe {
                for i in 0..epoch_size {
                    match operation {
                        Operation::Put => {
                            source[i].put_to(&mut dest[i], 1);
                        }
                        Operation::Get => {
                            dest[i].get_from(&mut source[i], 1);
                        }
                    }
                }
            }
            scope.barrier_all();

            if scope.my_pe() == 1 {
                for (i, (source, dest)) in source.iter().zip(dest.iter()).enumerate() {
                    // check if the data is correct
                    assert!(
                        source.iter().zip(dest.iter()).all(|(s, d)| s == d),
                        "Data mismatch at index {}: source = {}, dest = {}",
                        i,
                        source[i],
                        dest[i]
                    );
                }
            }
        }

        let elapsed = now.elapsed();

        let total_messages = epoch_per_iteration * epoch_size;
        let throughput = total_messages as f64 / elapsed.as_secs_f64();
        println!("Throughput on Machine {my_pe}: {:.2} messages/second", throughput);

        final_throughput = throughput;
    }

    final_throughput
}
