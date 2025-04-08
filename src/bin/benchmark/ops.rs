use clap::{Args, Parser, Subcommand};
use openshmem_benchmark::{osm_slice::OsmSlice, osm_wrapper::OsmWrapper};
use std::sync::atomic::AtomicU64;
use strum::{Display, EnumString};

#[derive(Subcommand, Debug, Clone, Copy, Display)]
pub enum Operation {
    #[command(flatten)]
    RangeOperation(RangeOperation),
    Atomic {
        #[command(subcommand)]
        op: AtomicOperation,
        #[arg(global = true, long, default_value_t = false)]
        use_different_location: bool,
    },
}

#[derive(Subcommand, Debug, Clone, Copy, Display)]

pub enum RangeOperation {
    #[command(flatten)]
    Put(PutOperation),
    #[command(flatten)]
    Get(GetOperation),
    Broadcast,
}

#[derive(Subcommand, Debug, Clone, Copy, Display)]
pub enum PutOperation {
    Put,
    PutNonBlocking,
}

#[derive(Subcommand, Debug, Clone, Copy, Display)]
pub enum GetOperation {
    Get,
    GetNonBlocking,
}

#[derive(Subcommand, Debug, Clone, Copy, Display)]
pub enum AtomicOperation {
    FetchAdd32,
    FetchAdd64,
}
