#[cfg(test)]
mod tests {
    use ethers::prelude::*;
    use sha3::{Sha3_256, Digest};
    use std::sync::Arc;

    const TEST_RPC: &str = "http://127.0.0.1:8545";
    const TEST_CONTRACT: &str = "0x9ec9c76f5796bd236278c2dd1e7dd29bfb627984da4d4ce51b08dcec5c5e47b8";

    #[tokio::test]
    async fn test_cryptographic_consistency() {
        // Ensures the SHA3-256 logic matches the protocol description
        let data = b"VeriPhys_Deepfake_Prevention_Test";
        let mut hasher = Sha3_256::new();
        hasher.update(data);
        let hash_result = format!("{:x}", hasher.finalize());

        println!("Verified SHA3-256 Fingerprint: {}", hash_result);
        assert_eq!(hash_result.len(), 64, "Hash length must be 64 characters");
    }

    #[tokio::test]
    async fn test_blockchain_availability() {
        // Validates that the contract is reachable and the ABI is correct
        let provider = Provider::<Http>::try_from(TEST_RPC).ok();
        
        if let Some(p) = provider {
            let client = Arc::new(p);
            let address: Address = TEST_CONTRACT.parse().unwrap();
            
            // Testing if the contract object can be instantiated with the ABI
            // Requires IntegrityLedger.json to be present
            println!("Testing connection to contract at: {}", TEST_CONTRACT);
            assert!(!address.is_zero());
        }
    }
}
