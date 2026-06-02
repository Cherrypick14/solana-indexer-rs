-- Initial schema for Solana Indexer
-- Creates all required tables for storing transaction data

-- Transactions table - stores basic transaction information
CREATE TABLE IF NOT EXISTS transactions (
    signature VARCHAR(88) PRIMARY KEY,         -- Base58 encoded transaction signature
    slot BIGINT NOT NULL,                      -- Slot number containing this transaction
    block_time BIGINT,                         -- Unix timestamp (optional, can be NULL)
    fee BIGINT NOT NULL,                       -- Transaction fee in lamports
    success BOOLEAN NOT NULL                   -- Transaction execution success status
);

-- Transaction accounts table - links transactions to account keys
CREATE TABLE IF NOT EXISTS transaction_accounts (
    transaction_signature VARCHAR(88) REFERENCES transactions(signature) ON DELETE CASCADE,
    account_key VARCHAR(44) NOT NULL,          -- Base58 encoded account public key
    account_index INTEGER NOT NULL,            -- Index of account in transaction
    PRIMARY KEY (transaction_signature, account_index)
);

-- Instructions table - stores transaction instructions with hierarchy support
CREATE TABLE IF NOT EXISTS instructions (
    id SERIAL PRIMARY KEY,                     -- Auto-incrementing instruction ID
    transaction_signature VARCHAR(88) REFERENCES transactions(signature) ON DELETE CASCADE,
    program_id VARCHAR(44) NOT NULL,           -- Target program public key
    data TEXT,                                 -- Base64 encoded instruction data
    parent_index INTEGER,                      -- Parent instruction index (for inner instructions)
    instruction_index INTEGER NOT NULL,        -- Index within transaction
    is_inner BOOLEAN NOT NULL DEFAULT FALSE    -- Whether this is an inner instruction
);

-- Instruction accounts table - links instructions to account keys
CREATE TABLE IF NOT EXISTS instruction_accounts (
    instruction_id INTEGER REFERENCES instructions(id) ON DELETE CASCADE,
    account_key VARCHAR(44) NOT NULL,          -- Base58 encoded account public key
    account_index INTEGER NOT NULL,            -- Index of account in instruction
    PRIMARY KEY (instruction_id, account_index)
);

-- Indexer state table - stores checkpoint and system state
CREATE TABLE IF NOT EXISTS indexer_state (
    key VARCHAR(50) PRIMARY KEY,               -- State key identifier
    value TEXT NOT NULL                        -- State value as text
);

-- Create indexes for efficient querying
CREATE INDEX IF NOT EXISTS idx_transactions_slot ON transactions(slot);
CREATE INDEX IF NOT EXISTS idx_transactions_block_time ON transactions(block_time);
CREATE INDEX IF NOT EXISTS idx_transaction_accounts_account_key ON transaction_accounts(account_key);
CREATE INDEX IF NOT EXISTS idx_instructions_transaction_signature ON instructions(transaction_signature);
CREATE INDEX IF NOT EXISTS idx_instructions_program_id ON instructions(program_id);
CREATE INDEX IF NOT EXISTS idx_instruction_accounts_account_key ON instruction_accounts(account_key);

-- Initialize indexer state with default last_indexed_slot
INSERT INTO indexer_state (key, value) 
VALUES ('last_indexed_slot', '0')
ON CONFLICT (key) DO NOTHING;