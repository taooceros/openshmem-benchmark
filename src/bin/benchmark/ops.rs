use clap::{Args, Parser, Subcommand, ValueEnum};
use openshmem_benchmark::{osm_slice::OsmSlice, osm_wrapper::OsmWrapper};
use serde::{Deserialize, Serialize};
use std::{path::{Path, PathBuf}, sync::atomic::AtomicU64};
use strum::{Display, EnumString};

#[derive(Subcommand, Debug, Clone, Display, PartialEq)]
pub enum Operation {
    #[command(subcommand)]
    #[strum(to_string = "Range({0})")]
    Range(RangeOperation),

    #[strum(to_string = "Atomic({op})")]
    Atomic {
        #[command(subcommand)]
        op: AtomicOperation,
        #[arg(global = true, long, default_value_t = false)]
        use_different_location: bool,
    },
}

#[derive(Subcommand, Debug, Clone, Display, PartialEq)]

pub enum RangeOperation {
    #[command(flatten)]
    #[strum(to_string = "{0}")]
    Put(PutOperation),
    #[command(flatten)]
    #[strum(to_string = "{0}")]
    Get(GetOperation),
    #[strum(to_string = "PutGet(Blocking={blocking})")]
    PutGet {
        #[arg(global = true, long)]
        put_ratio: Option<f64>,
        #[arg(global = true, long, value_delimiter = ',', num_args = 0..)]
        op_sequence: Option<Vec<OpenShmemOp>>,
        #[arg(global = true, long)]
        op_sequence_file: Option<String>,
        #[arg(global = true, long)]
        blocking: bool,
    },
    #[command(flatten)]
    #[strum(to_string = "{0}")]
    Broadcast(BroadcastOperation),

    AllToAll,
}

#[derive(ValueEnum, Serialize, Deserialize, Debug, Clone, Copy, Display, PartialEq)]
pub enum OpenShmemOp {
    Put,
    Get,
    Barrier,
    Fence,
}

#[derive(Subcommand, Debug, Clone, Copy, Display, PartialEq)]
pub enum BroadcastOperation {
    Broadcast,
}

#[derive(Subcommand, Debug, Clone, Copy, Display, PartialEq, Eq)]
pub enum PutOperation {
    Put,
    PutNonBlocking,
}

#[derive(Subcommand, Debug, Clone, Copy, Display, PartialEq, Eq)]
pub enum GetOperation {
    Get,
    GetNonBlocking,
}

#[derive(Subcommand, Debug, Clone, Copy, Display, PartialEq, Eq)]
pub enum PutGetOperation {
    PutGet,
    PutGetNonBlocking,
}

#[derive(Subcommand, Debug, Clone, Copy, Display, PartialEq, Eq)]
pub enum AtomicOperation {
    FetchAdd32,
    FetchAdd64,
    CompareAndSwap32,
    CompareAndSwap64,
}

pub(crate) fn read_op_sequence(unwrapped: &Path) -> Vec<OpenShmemOp> {
    // use serde read json file
    let file = std::fs::File::open(unwrapped).expect("Failed to open file");
    let reader = std::io::BufReader::new(file);
    let deserialized: Vec<OpenShmemOp> =
        serde_json::from_reader(reader).expect("Failed to deserialize JSON");

    return deserialized;
}
