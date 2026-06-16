/// Integration tests for Solana Indexer
/// These tests run against actual Solana networks to catch real-world edge cases
/// 
/// Run with:
/// cargo test --test integration_test -- --nocapture
/// 
/// To test against specific networks:
/// SOLANA_NETWORK=devnet cargo test --test integration_test
/// SOLANA_NETWORK=testnet cargo test --test integration_test
/// SOLANA_NETWORK=mainnet cargo test --test integration_test

use std::env;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SolanaNetwork {
    Devnet,
    Testnet,
    Mainnet,
}

impl SolanaNetwork {
    fn rpc_url(&self) -> &'static str {
        match self {
            SolanaNetwork::Devnet => "https://api.devnet.solana.com",
            SolanaNetwork::Testnet => "https://api.testnet.solana.com",
            SolanaNetwork::Mainnet => "https://api.mainnet-beta.solana.com",
        }
    }

    fn name(&self) -> &'static str {
        match self {
            SolanaNetwork::Devnet => "devnet",
            SolanaNetwork::Testnet => "testnet",
            SolanaNetwork::Mainnet => "mainnet",
        }
    }

    fn from_env() -> Self {
        match env::var("SOLANA_NETWORK")
            .unwrap_or_else(|_| "devnet".to_string())
            .as_str()
        {
            "testnet" => SolanaNetwork::Testnet,
            "mainnet" => SolanaNetwork::Mainnet,
            _ => SolanaNetwork::Devnet,
        }
    }
}

/// Test 1: Basic RPC Connectivity
/// **What we're testing:** Can we connect to the RPC endpoint?
/// **Why it matters:** If RPC is down, nothing works
/// **Expected behavior:** Quick response with current slot
#[tokio::test(flavor = "multi_thread")]
async fn test_rpc_connectivity() {
    let network = SolanaNetwork::from_env();
    println!("\n🔗 Testing RPC connectivity to {}", network.name());

    let client = solana_client::rpc_client::RpcClient::new(network.rpc_url());
    
    match client.get_slot() {
        Ok(slot) => {
            println!("✅ RPC Connection successful - Current slot: {}", slot);
            assert!(slot > 0, "Slot should be positive");
        }
        Err(e) => {
            println!("❌ RPC Connection failed: {}", e);
            panic!("Cannot connect to RPC endpoint");
        }
    }
}

/// Test 2: Block Fetching and Structure
/// **What we're testing:** Can we fetch a block and is the structure correct?
/// **Why it matters:** Different networks have different block sizes and transaction counts
/// **Edge cases:**
/// - Devnet: Usually has simple blocks, few programs
/// - Testnet: Medium activity, some real programs
/// - Mainnet: High activity, 10,000+ different programs
#[tokio::test(flavor = "multi_thread")]
async fn test_block_fetching() {
    let network = SolanaNetwork::from_env();
    println!("\n📦 Testing block fetching on {}", network.name());

    let client = solana_client::rpc_client::RpcClient::new(network.rpc_url());
    
    // Get the latest slot
    let latest_slot = client.get_slot().expect("Failed to get slot");
    println!("Latest slot: {}", latest_slot);

    // Fetch the block
    match client.get_block(latest_slot) {
        Ok(block) => {
            println!("✅ Block {} fetched", latest_slot);
            println!("   - Transactions: {}", block.transactions.len());
            println!("   - Parent slot: {:?}", block.parent_slot);
            println!("   - Block time: {:?}", block.block_time);

            // Validate block structure
            assert!(
                !block.transactions.is_empty() || network == SolanaNetwork::Devnet,
                "Block should have transactions (unless devnet with 0 activity)"
            );
        }
        Err(e) => {
            println!("❌ Failed to fetch block: {}", e);
            panic!("Cannot fetch block");
        }
    }
}

/// Test 3: Transaction Parsing
/// **What we're testing:** Can we correctly parse transaction details?
/// **Why it matters:** Different transaction types have different structures
/// **Edge cases to watch:**
/// - Failed transactions (success: false)
/// - Transactions with multiple instructions
/// - Inner instructions (program A calling program B)
/// - Transactions with account metadata
#[tokio::test(flavor = "multi_thread")]
async fn test_transaction_parsing() {
    let network = SolanaNetwork::from_env();
    println!("\n🔍 Testing transaction parsing on {}", network.name());

    let client = solana_client::rpc_client::RpcClient::new(network.rpc_url());
    
    let latest_slot = client.get_slot().expect("Failed to get slot");
    
    match client.get_block(latest_slot) {
        Ok(block) => {
            if block.transactions.is_empty() {
                println!("⚠️  No transactions in latest block, skipping parse test");
                return;
            }

            println!("✅ Block {} has {} transactions", latest_slot, block.transactions.len());
            
            // Count transactions with metadata
            let mut success_count = 0;
            let mut failed_count = 0;
            
            for tx in &block.transactions {
                if let Some(meta) = &tx.meta {
                    if meta.err.is_none() {
                        success_count += 1;
                    } else {
                        failed_count += 1;
                    }
                }
            }

            println!("   ✓ Successful transactions: {}", success_count);
            println!("   ✓ Failed transactions: {}", failed_count);

            // Validate that we can at least read transaction count
            assert!(
                !block.transactions.is_empty(),
                "Block should have transactions"
            );
        }
        Err(e) => {
            println!("❌ Failed to fetch block: {}", e);
            panic!("Cannot fetch block for parsing test");
        }
    }
}

/// Test 4: Network-Specific Edge Cases
/// **What we're testing:** Can we handle network-specific quirks?
/// **Why it matters:** Each network has unique characteristics
///
/// **Devnet quirks:**
/// - Low activity, sometimes 0 transactions/block
/// - Not all programs deployed
/// - Slot times vary (not consistent 400ms)
///
/// **Testnet quirks:**
/// - Medium activity, some real programs
/// - May have unusual program bytecode
///
/// **Mainnet quirks:**
/// - LOTS of activity, can be slow
/// - 10,000+ different programs
/// - Some programs have broken instructions
/// - Some transactions revert silently (success=true but instruction error)
#[tokio::test(flavor = "multi_thread")]
async fn test_network_characteristics() {
    let network = SolanaNetwork::from_env();
    println!("\n🌐 Analyzing {} characteristics", network.name());

    let client = solana_client::rpc_client::RpcClient::new(network.rpc_url());
    
    // Fetch last 5 blocks to get statistics
    let latest_slot = client.get_slot().expect("Failed to get slot");
    
    let mut transaction_counts = Vec::new();
    let mut failed_tx_count = 0;

    for i in 0..5 {
        let slot = latest_slot.saturating_sub(i);
        if let Ok(block) = client.get_block(slot) {
            transaction_counts.push(block.transactions.len());

            for tx in &block.transactions {
                if let Some(meta) = &tx.meta {
                    if meta.err.is_some() {
                        failed_tx_count += 1;
                    }
                }
            }
        }
    }

    let avg_txs = if !transaction_counts.is_empty() {
        transaction_counts.iter().sum::<usize>() / transaction_counts.len()
    } else {
        0
    };

    println!("📊 Statistics from last 5 blocks:");
    println!("   - Avg transactions/block: {}", avg_txs);
    println!("   - Failed transactions: {}", failed_tx_count);

    // Network-specific assertions
    match network {
        SolanaNetwork::Devnet => {
            // Devnet can have low activity
            println!("   ✓ Devnet: Low activity is normal");
        }
        SolanaNetwork::Testnet => {
            // Testnet should have moderate activity
            assert!(
                avg_txs > 0,
                "Testnet should have some transaction activity"
            );
            println!("   ✓ Testnet: Moderate activity confirmed");
        }
        SolanaNetwork::Mainnet => {
            // Mainnet should have significant activity
            assert!(
                avg_txs > 100,
                "Mainnet should have significant transaction activity (expected >100 tx/block)"
            );
            println!("   ✓ Mainnet: High activity confirmed");
        }
    }
}

/// Test 5: Error Handling
/// **What we're testing:** How do we handle real-world errors?
/// **Why it matters:** RPC providers are unreliable
///
/// **Common errors:**
/// - Timeout (slow RPC provider)
/// - Rate limiting (too many requests)
/// - Slot not available (slot was skipped or network reorg)
/// - Invalid parameters (using wrong encoding)
#[tokio::test(flavor = "multi_thread")]
async fn test_error_handling() {
    let network = SolanaNetwork::from_env();
    println!("\n⚠️  Testing error handling on {}", network.name());

    let client = solana_client::rpc_client::RpcClient::new(network.rpc_url());

    // Test 1: Request very old slot (may not exist)
    println!("   Testing request for very old slot...");
    match client.get_block(1) {
        Ok(block) => {
            println!("   ✓ Slot 1 exists, got {} transactions", block.transactions.len());
        }
        Err(e) => {
            println!("   ✓ Expected error for old slot: {}", e);
        }
    }

    // Test 2: Request future slot (doesn't exist yet)
    println!("   Testing request for future slot...");
    let future_slot = client.get_slot().expect("Failed to get slot") + 1000;
    match client.get_block(future_slot) {
        Ok(_) => {
            println!("   ⚠️  Got future slot (shouldn't happen)");
        }
        Err(_) => {
            println!("   ✓ Expected error for future slot");
        }
    }

    println!("   ✓ Error handling test completed");
}

/// Test 6: Retry Logic
/// **What we're testing:** Does our retry logic actually help?
/// **Why it matters:** RPC endpoints timeout sometimes
/// **Backoff strategy:** 1s, 2s, 4s = 7 seconds total
#[tokio::test(flavor = "multi_thread")]
async fn test_retry_behavior() {
    println!("\n🔄 Testing retry behavior");

    let mut attempt_count = 0;
    let result = retry_with_exponential_backoff(
        || {
            attempt_count += 1;
            println!("   Attempt {}", attempt_count);
            if attempt_count < 3 {
                Err("Simulated failure")
            } else {
                Ok("Success on attempt 3")
            }
        },
        3,
        std::time::Duration::from_millis(100), // Use shorter delays for testing
    );

    match result {
        Ok(msg) => {
            println!("   ✓ Retry succeeded: {}", msg);
            assert_eq!(attempt_count, 3, "Should have taken 3 attempts");
        }
        Err(e) => {
            println!("   ❌ Retry failed: {}", e);
        }
    }
}

// Helper function for retry testing
fn retry_with_exponential_backoff<F, T, E>(
    mut f: F,
    max_attempts: u32,
    base_delay: std::time::Duration,
) -> Result<T, E>
where
    F: FnMut() -> Result<T, E>,
{
    let mut last_error = None;

    for attempt in 1..=max_attempts {
        match f() {
            Ok(result) => return Ok(result),
            Err(e) => {
                last_error = Some(e);
                if attempt < max_attempts {
                    let delay = base_delay * 2_u32.pow(attempt - 1);
                    std::thread::sleep(delay);
                }
            }
        }
    }

    Err(last_error.unwrap())
}

// Summary of what each network teaches us:
// 
// DEVNET - The Sandbox
// ├─ Useful for: Development, testing basic functionality
// ├─ Activity: Low/sporadic
// ├─ Programs: Only what devs deploy
// └─ Lessons: Basic happy path works
//
// TESTNET - The Staging
// ├─ Useful for: Integration testing before mainnet
// ├─ Activity: Medium, consistent
// ├─ Programs: Mix of dev projects and real programs
// └─ Lessons: Basic error handling works
//
// MAINNET - The Real Deal
// ├─ Useful for: Production readiness verification
// ├─ Activity: Very high (200k+ tx/block)
// ├─ Programs: 10,000+ different programs
// ├─ Edge cases: Broken programs, silent failures, network chaos
// └─ Lessons: Need robust error handling, timeout management, rate limiting
