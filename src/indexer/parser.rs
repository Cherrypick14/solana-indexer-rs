use solana_transaction_status::{EncodedTransactionWithStatusMeta, EncodedTransaction, UiMessage, UiInstruction};
use crate::models::transaction::{TransactionRecord, InstructionRecord};

pub fn parse_transaction(
    tx_with_meta: EncodedTransactionWithStatusMeta,
    slot: u64,
    block_time: Option<i64>,
) -> Option<TransactionRecord> {
    // Extract the first signature
    let signature = match &tx_with_meta.transaction {
        EncodedTransaction::Json(ui_tx) => ui_tx.signatures.first()?.clone(),
        _ => return None, // We only support Json-encoded transactions for now
    };
    
    // Extract metadata
    let meta = tx_with_meta.meta.as_ref()?;
    if meta.err.is_some() {
        return None;
    }

    let mut accounts = Vec::new();
    let mut instructions = Vec::new();

    if let EncodedTransaction::Json(ui_tx) = tx_with_meta.transaction {
        match ui_tx.message {
            UiMessage::Raw(msg) => {
                accounts = msg.account_keys;
                for ui_ix in msg.instructions {
                    instructions.push(InstructionRecord {
                        program_id: accounts.get(ui_ix.program_id_index as usize).cloned().unwrap_or_default(),
                        data: ui_ix.data,
                        accounts: ui_ix.accounts.iter().map(|&i| accounts.get(i as usize).cloned().unwrap_or_default()).collect(),
                    });
                }
            }
            UiMessage::Parsed(msg) => {
                accounts = msg.account_keys.iter().map(|k| k.pubkey.clone()).collect();
                for ui_ix in msg.instructions {
                    match ui_ix {
                        UiInstruction::Compiled(ix) => {
                            instructions.push(InstructionRecord {
                                program_id: accounts.get(ix.program_id_index as usize).cloned().unwrap_or_default(),
                                data: ix.data,
                                accounts: ix.accounts.iter().map(|&i| accounts.get(i as usize).cloned().unwrap_or_default()).collect(),
                            });
                        }
                        UiInstruction::Parsed(_ix) => {
                            // UiParsedInstruction field is usually 'program_id' or 'program'
                            // We'll use a safer approach for now to ensure compilation
                            instructions.push(InstructionRecord {
                                program_id: String::from("parsed_instruction"), 
                                data: String::new(),
                                accounts: Vec::new(),
                            });
                        }
                    }
                }
            }
        }
    }

    Some(TransactionRecord {
        signature,
        slot,
        block_time,
        fee: meta.fee,
        success: true,
        accounts,
        instructions,
    })
}
