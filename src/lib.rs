// 1. Shared Data Models (Define these FIRST so they are available to modules)
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

// 2. Declare Modules
// We declare this after the structs so handlers can see them.
pub mod handlers;

/// Core Application State Definition
pub struct AppState {
    /// The generated Rust binding for the Solidity contract
    /// Note: We use the full path to avoid import confusion
    pub contract: handlers::VeriPhysContract<
        ethers::prelude::SignerMiddleware<
            ethers::prelude::Provider<ethers::prelude::Http>, 
            ethers::prelude::LocalWallet
        >
    >,
    pub registry_path: String,
    pub total_requests: std::sync::atomic::AtomicUsize,
}
