use serde::{Deserialize, Serialize};


#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "UPPERCASE")] 
pub enum OperationType {
    Put,
    PutNonBlocking,
    Get,
    GetNonBlocking,
    Barrier,
    Fence,
    FetchAdd32,
    FetchAdd64,
    CompareAndSwap32,
    CompareAndSwap64,
    AllToAll,
    AllReduce,
    AllGather,
    None,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Operation {
    pub op_type: OperationType,
    pub size: usize,
    #[serde(default)] 
    pub src: i32,
    #[serde(default)] 
    pub dst: i32,
}
