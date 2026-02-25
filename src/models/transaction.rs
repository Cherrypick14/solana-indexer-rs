use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TransactionRecord {
    pub signature: String,
    pub slot: u64,
    pub block_time: Option<i64>,
    pub fee: u64,
    pub success: bool,
    pub accounts: Vec<String>,
    pub instructions: Vec<InstructionRecord>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InstructionRecord {
    pub program_id: String,
    pub data: String,
    pub accounts: Vec<String>,
}
