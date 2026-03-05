// 1. Declare sub-modules
// This tells the compiler to look for handlers.rs in the same directory
pub mod handlers;

// 2. Re-export key types for cleaner imports in main.rs
pub use crate::handlers::{Record, IntegrityResponse, VeriPhysContract};

use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use ethers::prelude::*;

/// Core Application State
/// This structure is shared across all Axum routes and provides
/// thread-safe access to the blockchain client and global metrics.
pub struct AppState {
    /// The VeriPhys smart contract instance with signer middleware
    pub contract: handlers::VeriPhysContract<
        SignerMiddleware<Provider<Http>, LocalWallet>
    >,
    /// Path to the local text-based registry for audit logging
    pub registry_path: String,
    /// Atomic counter to track total anchor requests in real-time
    pub total_requests: AtomicUsize,
}

// Implement a helper to simplify State creation if needed
impl AppState {
    pub fn new(
        contract: handlers::VeriPhysContract<SignerMiddleware<Provider<Http>, LocalWallet>>,
        registry_path: String,
    ) -> Self {
        Self {
            contract,
            registry_path,
            total_requests: AtomicUsize::new(0),
        }
    }
}
