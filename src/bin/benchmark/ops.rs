use std::sync::atomic::AtomicU64;

use openshmem_benchmark::{osm_slice::OsmSlice, osm_wrapper::OsmWrapper};

#[derive(clap::ValueEnum, Debug, Clone, Copy)]
pub enum Operation {
    Put,
    Get,
    PutNonBlocking,
    GetNonBlocking,
    Broadcast,
    FetchAdd32,
    FetchAdd64,
}

impl ToString for Operation {
    fn to_string(&self) -> String {
        match self {
            Operation::Put => "put",
            Operation::Get => "get",
            Operation::PutNonBlocking => "put-non-blocking",
            Operation::GetNonBlocking => "get-non-blocking",
            Operation::Broadcast => "broadcast",
            Operation::FetchAdd32 => "fetch-add-32",
            Operation::FetchAdd64 => "fetch-add-64",
        }
        .to_string()
    }
}

impl Operation {
    pub fn get_operation_type(&self) -> OperationType {
        match self {
            Operation::Put => OperationType::RangeOperation(RangeOperation::Put),
            Operation::Get => OperationType::RangeOperation(RangeOperation::Get),
            Operation::PutNonBlocking => {
                OperationType::RangeOperation(RangeOperation::PutNonBlocking)
            }
            Operation::GetNonBlocking => {
                OperationType::RangeOperation(RangeOperation::GetNonBlocking)
            }
            Operation::Broadcast => OperationType::RangeOperation(RangeOperation::Broadcast),
            Operation::FetchAdd32 => OperationType::AtomicOperation(AtomicOperation::FetchAdd32),
            Operation::FetchAdd64 => OperationType::AtomicOperation(AtomicOperation::FetchAdd64),
        }
    }
}

pub enum RangeOperation {
    Put,
    Get,
    PutNonBlocking,
    GetNonBlocking,
    Broadcast,
}

pub enum AtomicOperation {
    FetchAdd32,
    FetchAdd64,
}

pub enum OperationType {
    RangeOperation(RangeOperation),
    AtomicOperation(AtomicOperation),
}
