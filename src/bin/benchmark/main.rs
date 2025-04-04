#![feature(allocator_api)]

use std::iter::repeat_with;
use std::ops::Deref;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Instant;

use bon::builder;
use clap::Parser;
use libc::gethostname;
use openshmem_benchmark::osm_box::OsmBox;
use openshmem_benchmark::osm_scope;
use openshmem_benchmark::osm_scope::OsmScope;
use openshmem_benchmark::osm_vec::ShVec;

mod layout;

use layout::BenchmarkData;

#[derive(clap::ValueEnum, Debug, Clone, Copy)]
enum Operation {
    Put,
    Get,
    PutNonBlocking,
    GetNonBlocking,
}

impl ToString for Operation {
    fn to_string(&self) -> String {
        match self {
            Operation::Put => "put".to_string(),
            Operation::Get => "get".to_string(),
            Operation::PutNonBlocking => "put-non-blocking".to_string(),
            Operation::GetNonBlocking => "get-non-blocking".to_string(),
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
    #[arg(short = 'w', long, default_value_t = 4)]
    num_working_set: usize,
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
    println!("  Operation: {}", config.operation.to_string());
    println!("  Number of Working Set: {}", config.num_working_set);
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

    let mut datas = repeat_with(|| {
        BenchmarkData::setup_data()
            .data_size(data_size)
            .epoch_size(epoch_size)
            .scope(&scope)
            .num_working_set(cli.num_working_set)
            .call()
    })
    .take(num_concurrency)
    .collect::<Vec<_>>();

    let my_pe = scope.my_pe() as usize % num_concurrency;
    let target_pe = my_pe;

    let final_throughput = benchmark_loop()
        .scope(&scope)
        .local_running(local_running.clone())
        .running(&mut running)
        .operation(operation)
        .epoch_per_iteration(cli.epoch_per_iteration)
        .data(&mut datas[target_pe])
        .call();

    println!("pe {}: stopping benchmark", scope.my_pe());

    output(&scope, num_concurrency, final_throughput, &cli);
}

fn output(scope: &OsmScope, num_concurrency: usize, final_throughput: f64, config: &Config) {
    // eprintln!("Final throughput: {:.2} messages/second", final_throughput);
    let my_pe = scope.my_pe() as usize;

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
            println!(
                "PE {}: {:.2} Gbps",
                i,
                throughputs[i].deref() * config.size as f64 / 1_000_000_000.0 * 8.0
            );
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
    data: &mut BenchmarkData<'a>,
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

    let false_signal = OsmBox::new(AtomicBool::new(false), &scope);

    loop {
        scope.barrier_all();

        if !running.load(std::sync::atomic::Ordering::SeqCst) {
            break;
        }

        let now = std::time::Instant::now();
        for epoch in 0..(epoch_per_iteration) {
            seed = (1 + seed * 7) % PRIME;
            let i = seed % num_working_set;

            let source = &mut data.src_working_set[i];
            let dest = &mut data.dst_working_set[i];

            let begin = Instant::now();
            for (src, dst) in source.iter_mut().zip(dest.iter_mut()) {
                match operation {
                    Operation::Put => {
                        if my_pe < num_concurrency {
                            src.put_to(dst, (my_pe + num_concurrency) as i32);
                        }
                    }
                    Operation::PutNonBlocking => {
                        if my_pe < num_concurrency {
                            src.put_to_nbi(dst, (my_pe + num_concurrency) as i32);
                        }
                    }
                    Operation::Get => {
                        if my_pe >= num_concurrency {
                            dst.get_from(src, (my_pe - num_concurrency) as i32);
                        }
                    }
                    Operation::GetNonBlocking => {
                        if my_pe >= num_concurrency {
                            dst.get_from_nbi(src, (my_pe - num_concurrency) as i32);
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

            let now = std::time::Instant::now();
            scope.barrier_all();
            if epoch % 1000 == 0 {
                // println!("pe {my_pe} {epoch} barrier elapsed time: {}", now.elapsed().as_micros());
            }

            // let now = std::time::Instant::now();

            if my_pe >= num_concurrency {
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

            // println!("elapsed time: {}", now.elapsed().as_micros());
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
