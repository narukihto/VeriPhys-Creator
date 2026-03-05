#[cfg(test)]
mod tests {
    use ethers::prelude::*;
    use sha3::{Sha3_256, Digest};
    use std::sync::Arc;
    use dotenv::dotenv;
    use std::env;

    /// Helper to initialize environment for tests
    fn init_env() {
        dotenv().ok();
    }

    #[tokio::test]
    async fn test_cryptographic_integrity_flow() {
        // 1. Simulate file data
        let data = b"VeriPhys_Protocol_Final_Consistency_Check";
        
        // 2. Generate SHA3-256 (Keccak-like but FIPS 202)
        let mut hasher = Sha3_256::new();
        hasher.update(data);
        let hash_bytes = hasher.finalize();
        let hash_array: [u8; 32] = hash_bytes.into();
        let hash_hex = hex::encode(hash_array);

        println!("Generated Hash (Hex): {}", hash_hex);
        
        // 3. Verify consistency for Blockchain bytes32
        assert_eq!(hash_array.len(), 32, "Binary hash must be exactly 32 bytes for Solidity");
        assert_eq!(hash_hex.len(), 64, "Hex string must be 64 characters");
    }

    #[tokio::test]
    async fn test_blockchain_connection_and_contract_load() {
        init_env();

        // Pulling real config from .env 
        let rpc_url = env::var("RPC_URL").expect("RPC_URL not set in .env");
        let contract_address_str = env::var("CONTRACT_ADDRESS").expect("CONTRACT_ADDRESS not set");

        let provider = Provider::<Http>::try_from(rpc_url)
            .expect("Failed to create provider");
        
        let address: Address = contract_address_str.parse()
            .expect("Failed to parse contract address");

        // Validate that the provider can reach the network
        let block_number = provider.get_block_number().await;
        
        assert!(block_number.is_ok(), "Blockchain node is unreachable. Is Anvil/Hardhat running?");
        assert!(!address.is_zero(), "Contract address cannot be the zero address");
        println!("Successfully connected to node. Current Block: {:?}", block_number.unwrap());
    }

    #[tokio::test]
    async fn test_registry_file_persistence() {
        init_env();
        let path = env::var("REGISTRY_PATH").unwrap_or_else(|_| "test_registry.txt".to_string());
        
        // Test async write consistency
        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .await
            .expect("Failed to open test registry");

        let test_entry = "test_file.png,d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2\n";
        let result = file.write_all(test_entry.as_bytes()).await;
        
        assert!(result.is_ok(), "Async IO failed to write to registry");
        
        // Cleanup: remove test file if desired
        // tokio::fs::remove_file(path).await.ok();
    }
}
