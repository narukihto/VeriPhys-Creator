use axum::{
    extract::{Multipart, State, DefaultBodyLimit},
    routing::{post, get},
    Json,
    Router,
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
        .expect("ADDR missing")
        .parse()
        .expect("Invalid Addr");
    let private_key = std::env::var("PRIVATE_KEY").expect("KEY missing");

    let provider = Provider::<Http>::try_from(rpc_url).unwrap();
    let wallet: LocalWallet = private_key.parse::<LocalWallet>()
        .unwrap()
        .with_chain_id(1337u64);
    let client = Arc::new(SignerMiddleware::new(provider, wallet));

    let shared_state = Arc::new(AppConfig {
        contract: VeriPhysContract::new(contract_addr, client),
        registry_path: std::env::var("REGISTRY_PATH").unwrap_or_else(|_| "registry.txt".to_string()),
        total_requests: AtomicUsize::new(0),
    });

    // 2. Protocol Router with Sentinel Security Layers
    let app = Router::new()
        .route("/v1/anchor", post(anchor_content))
        .route("/v1/registry", get(get_registry))
        .route("/v1/stats", get(get_stats))
        // High-security: Limit file uploads to 10MB to prevent DOS
        .layer(DefaultBodyLimit::max(10 * 1024 * 1024)) 
        .with_state(shared_state)
        .layer(tower_http::cors::CorsLayer::permissive());

    let port = std::env::var("SERVER_PORT").unwrap_or_else(|_| "3000".to_string());
    let addr: SocketAddr = format!("0.0.0.0:{}", port).parse().unwrap();

    println!("🛡️ VERIPHYS CORE: Infrastructure Online at {}", addr);
    println!("📊 Status: SHA3-256 + Blockchain Anchoring Active");
    
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

// --- API Handlers ---

async fn anchor_content(
    State(state): State<Arc<AppConfig>>,
    mut multipart: Multipart,
) -> Result<Json<IntegrityResponse>, (axum::http::StatusCode, String)> {
    // Track Metrics
    state.total_requests.fetch_add(1, Ordering::SeqCst);

    let mut file_name = String::from("unknown");
    let mut content_data = Vec::new();

    // Secure Multipart Extraction (Axum 0.7 Native)
    while let Ok(Some(field)) = multipart.next_field().await {
        if field.name() == Some("file") {
            file_name = field.file_name().unwrap_or("unnamed").to_string();
            content_data = field.bytes().await
                .map_err(|e| (axum::http::StatusCode::BAD_REQUEST, e.to_string()))?
                .to_vec();
        }
    }

    if content_data.is_empty() {
        return Err((axum::http::StatusCode::BAD_REQUEST, "Zero-byte file rejected".into()));
    }

    // 1. Digital Physics: SHA3-256 Fingerprinting
    let hash_bytes: [u8; 32] = Sha3_256::digest(&content_data).into();
    let file_hash_hex = hex::encode(hash_bytes);

    // 2. Immutable Anchoring: Blockchain Transaction
    let tx_receipt = state.contract
        .anchor_content(hash_bytes)
        .send()
        .await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, format!("Chain Error: {}", e)))?
        .await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, format!("TX Confirmation Failed: {}", e)))?;

    let tx_hash = match tx_receipt {
        Some(receipt) => format!("{:?}", receipt.transaction_hash),
        None => return Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, "No TX Confirmation".into())),
    };

    // 3. Local Audit: Async File Logging
    let log_entry = format!("{},{}\n", file_name, file_hash_hex);
    let mut file = OpenOptions::new()
        .create(true).append(true).open(&state.registry_path).await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    file.write_all(log_entry.as_bytes()).await.ok();

    Ok(Json(IntegrityResponse {
        status: "Success".into(),
        content_hash: file_hash_hex,
        tx_hash,
        message: "Digital fingerprint secured in the VeriPhys Ledger.".into(),
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
        "protocol_version": "1.0.0",
        "hashing_algorithm": "SHA3-256",
        "status": "Operational"
    }))
}
