use clap::{Args, Parser, Subcommand};
use openshmem_benchmark::{osm_slice::OsmSlice, osm_wrapper::OsmWrapper};
use std::sync::atomic::AtomicU64;
use strum::{Display, EnumString};

#[derive(Subcommand, Debug, Clone, Copy, Display, PartialEq, Eq)]
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

#[derive(Subcommand, Debug, Clone, Copy, Display, PartialEq, Eq)]

pub enum RangeOperation {
    #[command(flatten)]
    #[strum(to_string = "{0}")]
    Put(PutOperation),
    #[command(flatten)]
    #[strum(to_string = "{0}")]
    Get(GetOperation),
    #[command(flatten)]
    #[strum(to_string = "{0}")]
    Broadcast(BroadcastOperation),
}

#[derive(Subcommand, Debug, Clone, Copy, Display, PartialEq, Eq)]
pub enum BroadcastOperation {
    Broadcast,
    BroadcastNonBlocking,
    BroadcastLatency,
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
    GetLatency,
}

#[derive(Subcommand, Debug, Clone, Copy, Display, PartialEq, Eq)]
pub enum AtomicOperation {
    FetchAdd32,
    FetchAdd64,
}
