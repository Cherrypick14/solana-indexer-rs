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

#[cfg(test)]
mod tests {
    use super::*;
    use solana_transaction_status::{
        EncodedTransaction, EncodedTransactionWithStatusMeta,
        UiMessage, UiRawMessage, UiCompiledInstruction, UiTransactionStatusMeta,
        UiInnerInstructions, option_serializer::OptionSerializer,
        TransactionConfirmationStatus
    };
    use solana_sdk::transaction::TransactionError;

    fn create_test_transaction_with_meta(
        signatures: Vec<String>,
        account_keys: Vec<String>,
        instructions: Vec<UiCompiledInstruction>,
        inner_instructions: Option<Vec<UiInnerInstructions>>,
        has_error: bool,
        fee: u64,
    ) -> EncodedTransactionWithStatusMeta {
        let ui_transaction = solana_transaction_status::UiTransaction {
            signatures,
            message: UiMessage::Raw(UiRawMessage {
                header: solana_sdk::message::MessageHeader {
                    num_required_signatures: 1,
                    num_readonly_signed_accounts: 0,
                    num_readonly_unsigned_accounts: 1,
                },
                account_keys,
                recent_blockhash: "11111111111111111111111111111112".to_string(),
                instructions,
                address_table_lookups: None,
            }),
        };

        let meta = UiTransactionStatusMeta {
            err: if has_error { 
                Some(TransactionError::AccountInUse) 
            } else { 
                None 
            },
            status: Ok(()),
            fee,
            pre_balances: vec![1000000, 0, 1],
            post_balances: vec![999000, 1000, 1],
            inner_instructions: match inner_instructions {
                Some(ixs) => OptionSerializer::Some(ixs),
                None => OptionSerializer::None,
            },
            log_messages: OptionSerializer::Some(vec![]),
            pre_token_balances: OptionSerializer::None,
            post_token_balances: OptionSerializer::None,
            rewards: OptionSerializer::None,
            loaded_addresses: OptionSerializer::None,
            return_data: OptionSerializer::None,
            compute_units_consumed: OptionSerializer::None,
        };

        EncodedTransactionWithStatusMeta {
            transaction: EncodedTransaction::Json(ui_transaction),
            meta: Some(meta),
            version: None,
        }
    }

    #[test]
    fn test_signature_extraction_basic() {
        // Test basic signature extraction from a simple transaction
        let tx_with_meta = create_test_transaction_with_meta(
            vec!["5VfydDNssM4QFXwJjnM6DnFn7V1tJxn3TynHz9qu7LrdH8hsZ1WLKMtFtVscky5E4UFs6j5HE5F2WGH4mzYe2hKt".to_string()],
            vec!["11111111111111111111111111111112".to_string(), "4xTHj54G4tqQgMbx3xYmFQHxdMgcfXSVv4s4pZ8Ev6A1".to_string()],
            vec![UiCompiledInstruction {
                program_id_index: 1,
                accounts: vec![0],
                data: "Hello".to_string(),
                stack_height: None,
            }],
            None,
            false,
            5000,
        );

        let result = parse_transaction(tx_with_meta, 100, Some(1234567890));
        assert!(result.is_some());
        
        let tx_record = result.unwrap();
        assert_eq!(tx_record.signature, "5VfydDNssM4QFXwJjnM6DnFn7V1tJxn3TynHz9qu7LrdH8hsZ1WLKMtFtVscky5E4UFs6j5HE5F2WGH4mzYe2hKt");
        assert_eq!(tx_record.slot, 100);
        assert_eq!(tx_record.block_time, Some(1234567890));
        assert_eq!(tx_record.fee, 5000);
        assert_eq!(tx_record.success, true);
    }

    #[test]
    fn test_signature_extraction_multiple_signatures() {
        // Test signature extraction when there are multiple signatures (should use the first one)
        let tx_with_meta = create_test_transaction_with_meta(
            vec![
                "FirstSignature1111111111111111111111111111111111111111111111111111111".to_string(),
                "SecondSignature111111111111111111111111111111111111111111111111111111".to_string(),
            ],
            vec!["11111111111111111111111111111112".to_string(), "4xTHj54G4tqQgMbx3xYmFQHxdMgcfXSVv4s4pZ8Ev6A1".to_string()],
            vec![UiCompiledInstruction {
                program_id_index: 1,
                accounts: vec![0],
                data: "Hello".to_string(),
                stack_height: None,
            }],
            None,
            false,
            5000,
        );

        let result = parse_transaction(tx_with_meta, 100, None);
        assert!(result.is_some());
        
        let tx_record = result.unwrap();
        assert_eq!(tx_record.signature, "FirstSignature1111111111111111111111111111111111111111111111111111111");
    }

    #[test]
    fn test_signature_extraction_empty_signatures() {
        // Test handling of transaction with no signatures
        let tx_with_meta = create_test_transaction_with_meta(
            vec![], // Empty signatures
            vec!["11111111111111111111111111111112".to_string()],
            vec![],
            None,
            false,
            0,
        );

        let result = parse_transaction(tx_with_meta, 100, None);
        assert!(result.is_none()); // Should return None for empty signatures
    }

    #[test]
    fn test_failed_transaction_filtering() {
        // Test that transactions with errors are filtered out
        let tx_with_meta = create_test_transaction_with_meta(
            vec!["5VfydDNssM4QFXwJjnM6DnFn7V1tJxn3TynHz9qu7LrdH8hsZ1WLKMtFtVscky5E4UFs6j5HE5F2WGH4mzYe2hKt".to_string()],
            vec!["11111111111111111111111111111112".to_string(), "4xTHj54G4tqQgMbx3xYmFQHxdMgcfXSVv4s4pZ8Ev6A1".to_string()],
            vec![UiCompiledInstruction {
                program_id_index: 1,
                accounts: vec![0],
                data: "Hello".to_string(),
                stack_height: None,
            }],
            None,
            true, // Has error
            5000,
        );

        let result = parse_transaction(tx_with_meta, 100, None);
        assert!(result.is_none()); // Should return None for failed transactions
    }

    #[test]
    fn test_inner_instruction_parsing() {
        // Test parsing of inner instructions with correct parent relationships
        let inner_instructions = vec![
            UiInnerInstructions {
                index: 0, // Parent instruction index
                instructions: vec![
                    UiInstruction::Compiled(UiCompiledInstruction {
                        program_id_index: 1,
                        accounts: vec![0],
                        data: "InnerData".to_string(),
                        stack_height: None,
                    })
                ],
            }
        ];

        let tx_with_meta = create_test_transaction_with_meta(
            vec!["5VfydDNssM4QFXwJjnM6DnFn7V1tJxn3TynHz9qu7LrdH8hsZ1WLKMtFtVscky5E4UFs6j5HE5F2WGH4mzYe2hKt".to_string()],
            vec!["11111111111111111111111111111112".to_string(), "4xTHj54G4tqQgMbx3xYmFQHxdMgcfXSVv4s4pZ8Ev6A1".to_string()],
            vec![UiCompiledInstruction {
                program_id_index: 1,
                accounts: vec![0],
                data: "MainInstruction".to_string(),
                stack_height: None,
            }],
            Some(inner_instructions),
            false,
            5000,
        );

        let result = parse_transaction(tx_with_meta, 100, None);
        assert!(result.is_some());
        
        let tx_record = result.unwrap();
        assert_eq!(tx_record.instructions.len(), 2); // Main instruction + inner instruction

        // Check main instruction
        let main_instruction = &tx_record.instructions[0];
        assert_eq!(main_instruction.data, "MainInstruction");
        assert_eq!(main_instruction.is_inner, false);
        assert_eq!(main_instruction.parent_index, None);

        // Check inner instruction
        let inner_instruction = &tx_record.instructions[1];
        assert_eq!(inner_instruction.data, "InnerData");
        assert_eq!(inner_instruction.is_inner, true);
        assert_eq!(inner_instruction.parent_index, Some(0)); // Points to main instruction
    }

    #[test]
    fn test_inner_instruction_multiple_parents() {
        // Test parsing multiple inner instructions with different parents
        let inner_instructions = vec![
            UiInnerInstructions {
                index: 0, // Parent instruction index 0
                instructions: vec![
                    UiInstruction::Compiled(UiCompiledInstruction {
                        program_id_index: 1,
                        accounts: vec![0],
                        data: "Inner0".to_string(),
                        stack_height: None,
                    })
                ],
            },
            UiInnerInstructions {
                index: 1, // Parent instruction index 1
                instructions: vec![
                    UiInstruction::Compiled(UiCompiledInstruction {
                        program_id_index: 1,
                        accounts: vec![0],
                        data: "Inner1".to_string(),
                        stack_height: None,
                    })
                ],
            }
        ];

        let tx_with_meta = create_test_transaction_with_meta(
            vec!["5VfydDNssM4QFXwJjnM6DnFn7V1tJxn3TynHz9qu7LrdH8hsZ1WLKMtFtVscky5E4UFs6j5HE5F2WGH4mzYe2hKt".to_string()],
            vec!["11111111111111111111111111111112".to_string(), "4xTHj54G4tqQgMbx3xYmFQHxdMgcfXSVv4s4pZ8Ev6A1".to_string()],
            vec![
                UiCompiledInstruction {
                    program_id_index: 1,
                    accounts: vec![0],
                    data: "Main0".to_string(),
                    stack_height: None,
                },
                UiCompiledInstruction {
                    program_id_index: 1,
                    accounts: vec![0],
                    data: "Main1".to_string(),
                    stack_height: None,
                }
            ],
            Some(inner_instructions),
            false,
            5000,
        );

        let result = parse_transaction(tx_with_meta, 100, None);
        assert!(result.is_some());
        
        let tx_record = result.unwrap();
        assert_eq!(tx_record.instructions.len(), 4); // 2 main + 2 inner instructions

        // Verify parent relationships are correct
        let inner_with_parent_0 = tx_record.instructions.iter()
            .find(|ix| ix.data == "Inner0" && ix.parent_index == Some(0))
            .expect("Should find inner instruction with parent 0");
        assert!(inner_with_parent_0.is_inner);

        let inner_with_parent_1 = tx_record.instructions.iter()
            .find(|ix| ix.data == "Inner1" && ix.parent_index == Some(1))
            .expect("Should find inner instruction with parent 1");
        assert!(inner_with_parent_1.is_inner);
    }

    #[test]
    fn test_transaction_without_meta() {
        // Test handling of transaction without metadata
        let ui_transaction = solana_transaction_status::UiTransaction {
            signatures: vec!["5VfydDNssM4QFXwJjnM6DnFn7V1tJxn3TynHz9qu7LrdH8hsZ1WLKMtFtVscky5E4UFs6j5HE5F2WGH4mzYe2hKt".to_string()],
            message: UiMessage::Raw(UiRawMessage {
                header: solana_sdk::message::MessageHeader {
                    num_required_signatures: 1,
                    num_readonly_signed_accounts: 0,
                    num_readonly_unsigned_accounts: 1,
                },
                account_keys: vec!["11111111111111111111111111111112".to_string()],
                recent_blockhash: "11111111111111111111111111111112".to_string(),
                instructions: vec![],
                address_table_lookups: None,
            }),
        };

        let tx_with_meta = EncodedTransactionWithStatusMeta {
            transaction: EncodedTransaction::Json(ui_transaction),
            meta: None, // No metadata
            version: None,
        };

        let result = parse_transaction(tx_with_meta, 100, None);
        assert!(result.is_none()); // Should return None without metadata
    }

    #[quickcheck_macros::quickcheck]
    fn test_parser_correctness_property(
        signature: String,
        account1: String,
        account2: String,
        data: String,
        has_error: bool,
        fee: u64,
        slot: u64,
    ) -> bool {
        // Create an arbitrary transaction and ensure parser handles it without panicking
        let signatures = if signature.is_empty() { vec![] } else { vec![signature.clone()] };
        let accounts = vec![account1.clone(), account2.clone()];
        
        let tx_with_meta = create_test_transaction_with_meta(
            signatures,
            accounts,
            vec![UiCompiledInstruction {
                program_id_index: 1,
                accounts: vec![0],
                data: data.clone(),
                stack_height: None,
            }],
            None,
            has_error,
            fee,
        );

        let result = parse_transaction(tx_with_meta, slot, None);

        if signature.is_empty() || has_error {
            // Should be filtered out
            result.is_none()
        } else {
            // Should be parsed successfully
            if let Some(record) = result {
                record.signature == signature &&
                record.slot == slot &&
                record.fee == fee &&
                record.success &&
                record.instructions.len() == 1
            } else {
                false
            }
        }
    }
}