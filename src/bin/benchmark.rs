#![feature(allocator_api)]

use core::num;
use std::ops::Deref;
use std::process::exit;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64};

use bon::builder;
use clap::Parser;
use libc::{_SC_MEMORY_PROTECTION, gethostname};
use openshmem_benchmark::osm_arc::OsmArc;
use openshmem_benchmark::osm_box::OsmBox;
use openshmem_benchmark::osm_scope;
use openshmem_benchmark::osm_vec::ShVec;
use openshmem_benchmark::{osm_alloc::OsmMalloc, osm_scope::OsmScope};
use openshmem_sys::{my_pe, oshmem_team_world, shmem_char_p};

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
    #[arg(short, long, value_enum)]
    duration: Option<u64>,
    #[arg(short, long, default_value_t = Operation::Put)]
    operation: Operation,
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

#[builder]
fn setup_data<'a>(
    scope: &'a osm_scope::OsmScope,
    epoch_size: usize,
    data_size: usize,
) -> (Vec<ShVec<'a, u8>>, Vec<ShVec<'a, u8>>) {
    let mut source = Vec::with_capacity(epoch_size);
    let mut dest = Vec::with_capacity(epoch_size);

    for i in 0..epoch_size {
        let mut source_entry = ShVec::with_capacity(data_size, scope);
        let mut dest_entry = ShVec::with_capacity(data_size, scope);
        for j in 0..data_size {
            source_entry.push(((i * data_size) + j + 5) as u8);
        }
        dest_entry.resize_with(data_size, || 0);
        source.push(source_entry);
        dest.push(dest_entry);
    }

    (source, dest)
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
    println!("  Operation: {}", config.operation.to_string());
}

fn benchmark(cli: &Config) {
    let scope = osm_scope::OsmScope::init();

    print_config(cli, &scope);

    let local_running = setup_exit_signal(cli.duration, &scope);

    let mut running = OsmBox::new(AtomicBool::new(true), &scope);

    let operation = cli.operation;
    let epoch_size = cli.epoch_size;
    let data_size = cli.size;

    let num_pe = scope.num_pes();
    assert!(num_pe % 2 == 0, "Number of PEs must be even");
    let num_concurrency = (num_pe / 2) as usize;

    let mut sources = Vec::with_capacity(num_concurrency);
    let mut dests = Vec::with_capacity(num_concurrency);

    for _ in 0..num_concurrency {
        let (source, dest) = setup_data()
            .data_size(data_size)
            .epoch_size(epoch_size)
            .scope(&scope)
            .call();

        sources.push(source);
        dests.push(dest);
    }

    let my_pe = scope.my_pe() as usize % num_concurrency;
    let target_pe = my_pe;

    let final_throughput = benchmark_loop()
        .scope(&scope)
        .local_running(local_running.clone())
        .running(&mut running)
        .operation(operation)
        .epoch_per_iteration(cli.epoch_per_iteration)
        .epoch_size(epoch_size)
        .source(&mut sources[my_pe])
        .dest(&mut dests[target_pe])
        .call();

    println!("pe {}: stopping benchmark", scope.my_pe());

    output(&scope, num_concurrency, my_pe, final_throughput);
}

fn output(scope: &OsmScope, num_concurrency: usize, my_pe: usize, final_throughput: f64) {
    // eprintln!("Final throughput: {:.2} messages/second", final_throughput);

    let mut throughputs = ShVec::with_capacity(num_concurrency, &scope);

    throughputs.resize_with(num_concurrency, || 0.0);

    let my_throughput = OsmBox::new(final_throughput, &scope);

    // only sync half the pe
    if my_pe < num_concurrency {
        my_throughput.put_to_nbi(&mut throughputs[my_pe], 0);
    }

    // println!("pe {}: waiting for others", scope.my_pe());
    // sync all the pe
    scope.barrier_all();
    if scope.my_pe() == 0 {
        println!("Throughput on all PEs:");
        for i in 0..num_concurrency {
            eprintln!("PE {}: {:.2} messages/second", i, throughputs[i].deref());
        }
    }
}

#[builder]
fn benchmark_loop<'a>(
    scope: &osm_scope::OsmScope,
    local_running: Arc<AtomicBool>,
    running: &mut OsmBox<'a, AtomicBool>,
    operation: Operation,
    epoch_per_iteration: usize,
    epoch_size: usize,
    source: &mut Vec<ShVec<'a, u8>>,
    dest: &mut Vec<ShVec<'a, u8>>,
) -> f64 {
    let mut final_throughput = 0.0;
    let my_pe = scope.my_pe() as usize;
    let num_pe = scope.num_pes() as usize;
    let num_concurrency = (num_pe / 2) as usize;

    while running.load(std::sync::atomic::Ordering::Relaxed)
    {
        let now = std::time::Instant::now();
        for _ in 0..epoch_per_iteration {
            if my_pe < num_concurrency {
                for i in 0..epoch_size {
                    match operation {
                        Operation::Put => {
                            source[i].put_to(&mut dest[i], (my_pe + num_concurrency) as i32);
                        }
                        Operation::Get => {
                            dest[i].get_from(&mut source[i], (my_pe + num_concurrency) as i32);
                        }
                    }
                }
            }
            scope.barrier_all();

            if my_pe >= num_concurrency {
                for (i, (source, dest)) in source.iter().zip(dest.iter()).enumerate() {
                    // check if the data is correct
                    // eprintln!("source {:?}, dest {:?}", source, dest);
                    for (j, (left, right)) in source.iter().zip(dest.iter()).enumerate() {
                        if left != right {
                            panic!(
                                "Data mismatch at index {}: source {:?} dest {:?}",
                                j, left, right
                            );
                        }
                    }
                }
            }
        }

        let elapsed = now.elapsed();

        let total_messages = epoch_per_iteration * epoch_size;
        let throughput = total_messages as f64 / elapsed.as_secs_f64();
        // println!(
        //     "Throughput on Machine {my_pe}: {:.2} messages/second",
        //     throughput
        // );

        final_throughput = throughput;

        // let only the main pe to stop others
        if !local_running.load(std::sync::atomic::Ordering::Relaxed) && scope.my_pe() == 0 {
            // set the running flag to false
            let source = OsmBox::new(AtomicBool::new(false), &scope);
            println!("pe {}: stopping others", scope.my_pe());
            for i in 0..num_pe as i32 {
                source.put_to_nbi(running, i);
            }
        }

        scope.barrier_all();
    }

    final_throughput
}
