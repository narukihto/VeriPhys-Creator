use ethers::prelude::*;
use std::sync::Arc;
use tokio;

// Generate bindings for the test suite
abigen!(VeriPhysContract, "./IntegrityLedger.json");

#[tokio::test]
async fn test_blockchain_anchoring_flow() {
    // 1. Setup Mock/Local Provider (Anvil or Ganache)
    let provider = Provider::<Http>::try_from("http://localhost:8545").unwrap();
    let wallet: LocalWallet = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
        .parse::<LocalWallet>().unwrap();
    let client = Arc::new(SignerMiddleware::new(provider, wallet.with_chain_id(31337u64)));

    // 2. Mock Contract Address (Replace with actual deployed address for integration tests)
    let contract_address: Address = "0x5FbDB2315678afecb367f032d93F642f64180aa3".parse().unwrap();
    let contract = VeriPhysContract::new(contract_address, client);

    // 3. Test Data (SHA3-256 Hash Example)
    let test_hash: [u8; 32] = [0u8; 32]; // Mock hash for testing

    // 4. Execution: Anchor Content
    let tx = contract.anchor_content(test_hash).send().await;

    match tx {
        Ok(pending_tx) => {
            let receipt = pending_tx.await.unwrap();
            assert!(receipt.is_some(), "Transaction should be mined");
            println!("✅ Anchoring successful: {:?}", receipt.unwrap().transaction_hash);
        },
        Err(e) => {
            if e.to_string().contains("HashAlreadyAnchored") {
                println!("ℹ️ Hash already exists (Expected behavior for duplicates)");
            } else {
                panic!("❌ Unexpected blockchain error: {}", e);
            }
        }
    }
}
