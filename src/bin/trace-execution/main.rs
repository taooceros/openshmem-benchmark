pub mod operations;

use std::{fs::File, io::BufReader};

use clap::Parser;

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

    execution::run(operations);
}
