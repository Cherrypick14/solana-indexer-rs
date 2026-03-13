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
        _ => return None, // Only  supports Json-encoded transactions for now
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
                for (idx, ui_ix) in msg.instructions.iter().enumerate() {
                    instructions.push(InstructionRecord {
                        program_id: accounts.get(ui_ix.program_id_index as usize).cloned().unwrap_or_default(),
                        data: ui_ix.data.clone(),
                        accounts: ui_ix.accounts.iter().map(|&i| accounts.get(i as usize).cloned().unwrap_or_default()).collect(),
                        parent_index: None,
                        is_inner: false,
                    });

                    // Add inner instructions if any
                    let inner_instructions = match meta.inner_instructions.as_ref() {
                        solana_transaction_status::option_serializer::OptionSerializer::Some(ixs) => Some(ixs.clone()),
                        _ => None,
                    };

                    if let Some(inner_ixs) = inner_instructions {
                        if let Some(inner) = inner_ixs.iter().find(|i| i.index == idx as u8) {
                            for ui_inner in &inner.instructions {
                                let (program_id, data, accounts) = match ui_inner {
                                    UiInstruction::Compiled(ix) => (
                                        accounts.get(ix.program_id_index as usize).cloned().unwrap_or_default(),
                                        ix.data.clone(),
                                        ix.accounts.iter().map(|&i| accounts.get(i as usize).cloned().unwrap_or_default()).collect(),
                                    ),
                                    UiInstruction::Parsed(ix) => {
                                        let program_id = match ix {
                                            solana_transaction_status::UiParsedInstruction::Parsed(p) => p.program_id.clone(),
                                            solana_transaction_status::UiParsedInstruction::PartiallyDecoded(p) => p.program_id.clone(),
                                        };
                                        let data = match ix {
                                            solana_transaction_status::UiParsedInstruction::Parsed(p) => p.parsed.to_string(),
                                            solana_transaction_status::UiParsedInstruction::PartiallyDecoded(p) => p.data.clone(),
                                        };
                                        (program_id, data, Vec::new())
                                    }
                                };

                                instructions.push(InstructionRecord {
                                    program_id,
                                    data,
                                    accounts,
                                    parent_index: Some(idx),
                                    is_inner: true,
                                });
                            }
                        }
                    }
                }
            }
            UiMessage::Parsed(msg) => {
                accounts = msg.account_keys.iter().map(|k| k.pubkey.clone()).collect();
                for (idx, ui_ix) in msg.instructions.iter().enumerate() {
                    match ui_ix {
                        UiInstruction::Compiled(ix) => {
                            instructions.push(InstructionRecord {
                                program_id: accounts.get(ix.program_id_index as usize).cloned().unwrap_or_default(),
                                data: ix.data.clone(),
                                accounts: ix.accounts.iter().map(|&i| accounts.get(i as usize).cloned().unwrap_or_default()).collect(),
                                parent_index: None,
                                is_inner: false,
                            });
                        }
                        UiInstruction::Parsed(ix) => {
                            // Extract program ID and data from parsed instruction
                            let program_id = match ix {
                                solana_transaction_status::UiParsedInstruction::Parsed(p) => p.program_id.clone(),
                                solana_transaction_status::UiParsedInstruction::PartiallyDecoded(p) => p.program_id.clone(),
                            };
                            
                            let data = match ix {
                                solana_transaction_status::UiParsedInstruction::Parsed(p) => p.parsed.to_string(),
                                solana_transaction_status::UiParsedInstruction::PartiallyDecoded(p) => p.data.clone(),
                            };

                            instructions.push(InstructionRecord {
                                program_id,
                                data,
                                accounts: Vec::new(), // Parsed instructions usually have accounts in the 'parsed' JSON
                                parent_index: None,
                                is_inner: false,
                            });
                        }
                    }

                    // Add inner instructions if any
                    let inner_instructions = match meta.inner_instructions.as_ref() {
                        solana_transaction_status::option_serializer::OptionSerializer::Some(ixs) => Some(ixs.clone()),
                        _ => None,
                    };

                    if let Some(inner_ixs) = inner_instructions {
                        if let Some(inner) = inner_ixs.iter().find(|i| i.index == idx as u8) {
                            for ui_inner in &inner.instructions {
                                let (program_id, data, accounts) = match ui_inner {
                                    UiInstruction::Compiled(ix) => (
                                        accounts.get(ix.program_id_index as usize).cloned().unwrap_or_default(),
                                        ix.data.clone(),
                                        ix.accounts.iter().map(|&i| accounts.get(i as usize).cloned().unwrap_or_default()).collect(),
                                    ),
                                    UiInstruction::Parsed(ix) => {
                                        let program_id = match ix {
                                            solana_transaction_status::UiParsedInstruction::Parsed(p) => p.program_id.clone(),
                                            solana_transaction_status::UiParsedInstruction::PartiallyDecoded(p) => p.program_id.clone(),
                                        };
                                        let data = match ix {
                                            solana_transaction_status::UiParsedInstruction::Parsed(p) => p.parsed.to_string(),
                                            solana_transaction_status::UiParsedInstruction::PartiallyDecoded(p) => p.data.clone(),
                                        };
                                        (program_id, data, Vec::new())
                                    }
                                };

                                instructions.push(InstructionRecord {
                                    program_id,
                                    data,
                                    accounts,
                                    parent_index: Some(idx),
                                    is_inner: true,
                                });
                            }
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
