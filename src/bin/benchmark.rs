#![feature(allocator_api)]

use std::ops::Deref;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use clap::Parser;
use openshmem_benchmark::osm_alloc::OsmMalloc;
use openshmem_benchmark::osm_scope;
use openshmem_benchmark::osm_vec::ShVec;
use openshmem_sys::shmem_char_p;

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

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Config {
    #[arg(short, long, default_value_t = 1)]
    epoch_size: usize,
    #[arg(short, long, default_value_t = 1)]
    size: usize,
    #[arg(short, long, default_value_t = 1000000)]
    epoch_per_iteration: usize,
    #[arg(short='p', long, default_value_t = 1)]
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

fn setup_data<'a>(
    scope: &'a osm_scope::OsmScope,
    window_size: usize,
    data_size: usize,
) -> (Vec<ShVec<'a, u8>>, Vec<ShVec<'a, u8>>) {
    let mut source = Vec::with_capacity(window_size);
    let mut dest = Vec::with_capacity(window_size);

    for i in 0..window_size {
        source.push(ShVec::with_capacity(data_size, scope));
        for j in 0..data_size {
            source[i].push((i * data_size + j) as u8);
        }
        dest.push(ShVec::with_capacity(data_size, scope));
        for j in 0..data_size {
            dest[i].push(0);
        }
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

    let running = std::sync::Arc::new_in(
        std::sync::atomic::AtomicBool::new(true),
        OsmMalloc::new(&scope),
    );

    let mut final_throughput = 0.0;

    let operation = cli.operation;
    let epoch_size = cli.epoch_size;
    let data_size = cli.size;

    let end = Box::new_in(true, OsmMalloc::new(&scope));

    let (mut source, mut dest) = setup_data(&scope, epoch_size, data_size);

    'outer: while running.load(std::sync::atomic::Ordering::Relaxed)
        && local_running.load(std::sync::atomic::Ordering::Relaxed)
    {
        let now = std::time::Instant::now();
        for _ in 0..cli.epoch_per_iteration {
            if !running.load(std::sync::atomic::Ordering::Relaxed) {
                break 'outer;
            }

            if scope.my_pe() == 0 {
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

        let total_messages = cli.epoch_per_iteration * epoch_size;
        let throughput = total_messages as f64 / elapsed.as_secs_f64();
        println!("Throughput: {:.2} messages/second", throughput);

        final_throughput = throughput;
    }

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
