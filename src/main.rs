use axum::{
    extract::DefaultBodyLimit,
    routing::{post, get},
    Router,
};
use std::sync::Arc;
use std::net::SocketAddr;
use ethers::prelude::*;
use dotenvy::dotenv;

// --- FIXED: Do NOT use 'mod handlers;' here ---
// We import everything through the library name 'veriphys_protocol_core'
use veriphys_protocol_core::{AppState, handlers as h};

#[tokio::main]
async fn main() {
    dotenv().ok();

    // 1. Setup Blockchain Provider & Signer
    let rpc_url = std::env::var("RPC_URL").expect("RPC_URL must be set");
    let contract_addr: Address = std::env::var("CONTRACT_ADDRESS")
        .expect("CONTRACT_ADDRESS must be set")
        .parse()
        .expect("Invalid Contract Address");
    let private_key = std::env::var("PRIVATE_KEY").expect("PRIVATE_KEY must be set");

    let provider = Provider::<Http>::try_from(rpc_url).expect("Failed to connect to RPC");
    
    // Dynamically get Chain ID or default to Anvil (31337)
    let wallet: LocalWallet = private_key.parse::<LocalWallet>()
        .expect("Invalid Private Key")
        .with_chain_id(31337u64); 

    let client = Arc::new(SignerMiddleware::new(provider, wallet));
    
    // Initialize Contract Binding from the handlers module re-exported by your lib
    let contract = h::VeriPhysContract::new(contract_addr, client);

    // 2. Initialize Shared State
    let shared_state = Arc::new(AppState {
        contract,
        registry_path: std::env::var("REGISTRY_PATH").unwrap_or_else(|_| "registry.txt".into()),
        total_requests: std::sync::atomic::AtomicUsize::new(0),
    });

    // 3. Build API Router
    let app = Router::new()
        .route("/v1/anchor", post(h::anchor_content))
        .route("/v1/registry", get(h::get_registry))
        .route("/v1/stats", get(h::get_stats))
        .layer(DefaultBodyLimit::max(10 * 1024 * 1024)) // 10MB Limit
        .with_state(shared_state);

    // 4. Launch Server
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    println!("🛡️ VeriPhys Core Engine started on {}", addr);
    
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
