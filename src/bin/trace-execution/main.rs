pub mod operations;

use std::{fs::File, io::BufReader};

use clap::Parser;
use openshmem_benchmark::osm_scope;

use crate::operations::Operation;

#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long)]
    trace_file: String,
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

    let mut times = Vec::new();
    for trial in 1..10 {
        let time = execution::run(&operations, &scope);
        println!("Trial {}: {}", trial, time);
        times.push(time);
    }

    let throughput = operations.len() as f64 / times.iter().sum::<f64>() / times.len() as f64;
    println!("Op/s average: {}", throughput);
}
