pub mod operations;

use std::{fs::File, io::BufReader};

use clap::Parser;
use openshmem_benchmark::osm_scope;

use crate::operations::Operation;

#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long)]
    trace_file: String,
    #[arg(short, long)]
    small_message: bool,
}

pub mod execution;

fn main() {
    let args = Args::parse();
    println!("Trace file: {}", args.trace_file);

    let trace_file = File::open(args.trace_file).unwrap();
    let reader = BufReader::new(trace_file);
    let operations = csv::Reader::from_reader(reader)
        .deserialize::<Operation>()
        .map(|e| e.unwrap())
        .collect::<Vec<_>>();
    let scope = osm_scope::OsmScope::init();

    let min_sec = 10.0;
    let mut num_ops = 0;
    let mut times = Vec::new();
    loop {
        let (each_num_ops, time) = execution::run(&operations, &scope);
        println!("Trial {}: {}", times.len(), time);
        println!("current Op/s (in {:0.2}s): {:0.2}", time, each_num_ops as f64 / time);
        println!("Num ops: {}", each_num_ops);
        times.push(time);
        if times.iter().sum::<f64>() >= min_sec {
            break;
        }
        num_ops += each_num_ops;
    }

    let throughput = num_ops as f64 / (times.iter().sum::<f64>() / times.len() as f64);
    println!("Op/s: {}", throughput);
    eprintln!("Num ops: {}", num_ops);
    eprintln!("Times: {:?}", times);
    eprintln!("Throughput: {}", throughput);
}
