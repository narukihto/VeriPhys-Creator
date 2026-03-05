// 1. Declare Modules
// This tells Rust that the file 'handlers.rs' exists and is part of this library.
pub mod handlers;

// 2. Re-export for easier access
// This allows other files to use 'AppState' directly without long paths.
pub use crate::handlers::VeriPhysContract;

/// Core Application State Definition
/// This struct holds the blockchain client and system configurations 
/// shared across all API routes.
pub struct AppState {
    /// The generated Rust binding for the Solidity contract
    pub contract: handlers::VeriPhysContract<
        ethers::prelude::SignerMiddleware<
            ethers::prelude::Provider<ethers::prelude::Http>, 
            ethers::prelude::LocalWallet
        >
    >,
    /// System path for the local audit trail (registry.txt)
    pub registry_path: String,
    /// Atomic counter for monitoring total protocol interactions
    pub total_requests: std::sync::atomic::AtomicUsize,
}

// 3. Shared Data Models
// These are used by both handlers and the main server for JSON serialization.
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct Record {
    pub file_name: String,
    pub file_hash: String,
}

#[derive(Serialize)]
pub struct IntegrityResponse {
    pub status: String,
    pub content_hash: String,
    pub tx_hash: String,
    pub message: String,
}
