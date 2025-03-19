#![feature(allocator_api)]
use std::ffi::c_void;

use clap::Parser;
use openshmem_sys::*;
use shalloc::ShMalloc;

mod shalloc;

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
    window_size: usize,
    #[arg(short, long, default_value_t = 1)]
    size: usize,
    #[arg(short, long, default_value_t = 1000000)]
    num_iterations: usize,
    #[arg(short, long, value_enum)]
    duration: Option<u64>,
    #[arg(short, long, default_value_t = Operation::Put)]
    operation: Operation,
}

fn main() {
    let config = Config::parse();

    println!(
        "Benchmarking OpenSHMEM with window size: {} and data size: {} with {} iterations",
        config.window_size, config.size, config.num_iterations
    );

    benchmark(&config);
}

fn benchmark(cli: &Config) {
    unsafe {
        shmem_init();

        let running = std::sync::Arc::new_in(std::sync::atomic::AtomicBool::new(true), ShMalloc);
        let r1 = running.clone();
        let r2 = running.clone();
        ctrlc::set_handler(move || {
            r1.store(false, std::sync::atomic::Ordering::Relaxed);
        })
        .expect("Error setting Ctrl-C handler");

        let mut final_throughput = 0.0;

        if shmem_my_pe() == 0 {
            if let Some(duration) = cli.duration {
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_secs(duration));
                    r2.store(false, std::sync::atomic::Ordering::Relaxed);
                });
            }
        }

        let operation = cli.operation;
        let window_size = cli.window_size;
        let data_size = cli.size;

        let mut source = Vec::with_capacity(window_size);
        let mut dest = Vec::with_capacity(window_size);

        let end = Box::new_in(true, ShMalloc);

        for i in 0..window_size {
            source.push(Vec::with_capacity_in(data_size, ShMalloc));
            for j in 0..data_size {
                source[i].push((i * data_size + j) as u8);
            }
            dest.push(Vec::with_capacity_in(data_size, ShMalloc));
            for j in 0..data_size {
                dest[i].push(0);
            }
        }

        'outer: while running.load(std::sync::atomic::Ordering::Relaxed) {
            let now = std::time::Instant::now();
            for _ in 0..cli.num_iterations {
                if !running.load(std::sync::atomic::Ordering::Relaxed) {
                    break 'outer;
                }

                if shmem_my_pe() == 0 {
                    for i in 0..window_size {
                        match operation {
                            Operation::Put => {
                                shmem_putmem(
                                    dest[i].as_mut_ptr() as *mut c_void,
                                    source[i].as_ptr() as *const c_void,
                                    data_size,
                                    1,
                                );
                            }
                            Operation::Get => {
                                shmem_getmem(
                                    dest[i].as_mut_ptr() as *mut c_void,
                                    source[i].as_ptr() as *const c_void,
                                    data_size,
                                    1,
                                );
                            }
                        }
                    }
                }
                shmem_barrier_all();
            }

            let elapsed = now.elapsed();

            let total_messages = cli.num_iterations * window_size;
            let throughput = total_messages as f64 / elapsed.as_secs_f64();
            println!("Throughput: {:.2} messages/second", throughput);

            final_throughput = throughput;
        }

        shmem_char_p(running.as_ptr() as *mut i8, true as i8, 1);
        shmem_barrier_all();
        eprintln!("Final throughput: {:.2} messages/second", final_throughput);
    }

    unsafe {
        println!("Finalizing OpenSHMEM");
        shmem_finalize();
    }
}
