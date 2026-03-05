use axum::{
    extract::{Multipart, State, DefaultBodyLimit},
    routing::{post, get},
    Json,
    Router,
    http::StatusCode,
};
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::net::SocketAddr;
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;
use ethers::prelude::*;
use sha3::{Sha3_256, Digest};
use dotenvy::dotenv;

// Generate Rust bindings from the Solidity ABI
// Ensure IntegrityLedger.json is in your project root
abigen!(VeriPhysContract, "./IntegrityLedger.json");

/// Protocol Global State
struct AppConfig {
    contract: VeriPhysContract<SignerMiddleware<Provider<Http>, LocalWallet>>,
    registry_path: String,
    total_requests: AtomicUsize,
}

#[derive(Serialize, Deserialize, Clone)]
struct Record {
    file_name: String,
    file_hash: String,
}

#[derive(Serialize)]
struct IntegrityResponse {
    status: String,
    content_hash: String,
    tx_hash: String,
    message: String,
}

#[tokio::main]
async fn main() {
    // 1. Initialize Secure Environment
    dotenv().ok();

    let rpc_url = std::env::var("RPC_URL").expect("RPC_URL missing");
    let contract_addr: Address = std::env::var("CONTRACT_ADDRESS")
        .expect("CONTRACT_ADDRESS missing")
        .parse()
        .expect("Invalid Address format");
    let private_key = std::env::var("PRIVATE_KEY").expect("PRIVATE_KEY missing");

    let provider = Provider::<Http>::try_from(rpc_url)
        .expect("Could not instantiate HTTP Provider");
    
    // Dynamic Chain ID detection to prevent replay protection errors
    let chain_id = provider.get_chainid().await.unwrap_or(U256::from(1337)).as_u64();
    
    let wallet: LocalWallet = private_key.parse::<LocalWallet>()
        .expect("Invalid Private Key")
        .with_chain_id(chain_id);
    
    let client = Arc::new(SignerMiddleware::new(provider, wallet));

    let shared_state = Arc::new(AppConfig {
        contract: VeriPhysContract::new(contract_addr, client),
        registry_path: std::env::var("REGISTRY_PATH").unwrap_or_else(|_| "registry.txt".to_string()),
        total_requests: AtomicUsize::new(0),
    });

    // 2. Protocol Router with Modern Middleware
    let app = Router::new()
        .route("/v1/anchor", post(anchor_content))
        .route("/v1/registry", get(get_registry))
        .route("/v1/stats", get(get_stats))
        // Security Layer: Prevent Large File Attacks (10MB Limit)
        .layer(DefaultBodyLimit::max(10 * 1024 * 1024)) 
        .with_state(shared_state)
        .layer(tower_http::cors::CorsLayer::permissive());

    let port = std::env::var("SERVER_PORT").unwrap_or_else(|_| "3000".to_string());
    let addr: SocketAddr = format!("0.0.0.0:{}", port).parse().unwrap();

    println!("🛡️ VERIPHYS CORE: Infrastructure Online at {}", addr);
    println!("⛓️ Connected to Chain ID: {}", chain_id);
    
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

// --- API Handlers ---

async fn anchor_content(
    State(state): State<Arc<AppConfig>>,
    mut multipart: Multipart,
) -> Result<Json<IntegrityResponse>, (StatusCode, String)> {
    state.total_requests.fetch_add(1, Ordering::SeqCst);

    let mut file_name = String::from("unknown");
    let mut content_data = Vec::new();

    // Secure Multipart Extraction (Optimized for Axum 0.7)
    while let Some(field) = multipart.next_field().await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))? 
    {
        if field.name() == Some("file") {
            file_name = field.file_name().unwrap_or("unnamed").to_string();
            content_data = field.bytes().await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
                .to_vec();
        }
    }

    if content_data.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "File is empty or missing".into()));
    }

    // 1. Digital Physics: SHA3-256 Hashing
    let hash_bytes: [u8; 32] = Sha3_256::digest(&content_data).into();
    let file_hash_hex = hex::encode(hash_bytes);

    // 2. Blockchain Anchoring
    // We send hash_bytes directly to the Solidity 'bytes32' input
    let tx = state.contract.anchor_content(hash_bytes);
    
    let pending_tx = tx.send().await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Blockchain Send Error: {}", e)))?;

    let receipt = pending_tx.await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Mining Error: {}", e)))?
        .ok_or((StatusCode::INTERNAL_SERVER_ERROR, "Transaction dropped".to_string()))?;

    let tx_hash = format!("{:?}", receipt.transaction_hash);

    // 3. Local Audit Log
    let log_entry = format!("{},{}\n", file_name, file_hash_hex);
    let mut file = OpenOptions::new()
        .create(true).append(true).open(&state.registry_path).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("IO Error: {}", e)))?;
    
    file.write_all(log_entry.as_bytes()).await.ok();

    Ok(Json(IntegrityResponse {
        status: "Success".into(),
        content_hash: file_hash_hex,
        tx_hash,
        message: "Fingerprint anchored to the distributed ledger.".into(),
    }))
}

async fn get_registry(State(state): State<Arc<AppConfig>>) -> Json<Vec<Record>> {
    let content = tokio::fs::read_to_string(&state.registry_path).await.unwrap_or_default();
    let records = content.lines()
        .filter_map(|line| {
            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() == 2 {
                Some(Record { file_name: parts[0].into(), file_hash: parts[1].into() })
            } else { None }
        }).collect();
    Json(records)
}

async fn get_stats(State(state): State<Arc<AppConfig>>) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "total_anchors": state.total_requests.load(Ordering::SeqCst),
        "protocol_version": "1.1.0",
        "engine": "VeriPhys-Rust-Axum0.7",
        "status": "Operational"
    }))
}
