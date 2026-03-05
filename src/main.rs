use axum::{extract::{Multipart, State}, routing::{post, get}, Json, Router};
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use std::net::SocketAddr;
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;
use ethers::prelude::*;
use sha3::{Sha3_256, Digest};
// Swapped to maintained crate
use dotenvy::dotenv; 

// Generate Rust bindings from the corrected ABI
abigen!(VeriPhysContract, "./IntegrityLedger.json");

#[derive(Clone)]
struct AppConfig {
    contract: VeriPhysContract<SignerMiddleware<Provider<Http>, LocalWallet>>,
    registry_path: String,
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
    // 1. Load environment using secure dotenvy
    dotenv().ok(); 

    // Setup Blockchain Provider & Wallet
    let rpc_url = std::env::var("RPC_URL").expect("RPC_URL must be set"); 
    let contract_addr: Address = std::env::var("CONTRACT_ADDRESS").expect("ADDR missing").parse().expect("Invalid Addr"); 
    let private_key = std::env::var("PRIVATE_KEY").expect("KEY missing"); 
    
    let provider = Provider::<Http>::try_from(rpc_url).unwrap();
    // Defaulting to local network (Anvil/Hardhat)
    let wallet: LocalWallet = private_key.parse::<LocalWallet>().unwrap().with_chain_id(1337u64);
    let client = Arc::new(SignerMiddleware::new(provider, wallet));
    
    let shared_state = Arc::new(AppConfig {
        contract: VeriPhysContract::new(contract_addr, client),
        registry_path: std::env::var("REGISTRY_PATH").unwrap_or_else(|_| "registry.txt".to_string()), 
    });

    // 2. Build Router with State and standard CORS
    let app = Router::new()
        .route("/v1/anchor", post(anchor_content))
        .route("/v1/registry", get(get_registry))
        .with_state(shared_state)
        .layer(tower_http::cors::CorsLayer::permissive());

    let port = std::env::var("SERVER_PORT").unwrap_or_else(|_| "3000".to_string()); 
    let addr: SocketAddr = format!("0.0.0.0:{}", port).parse().unwrap();
    
    println!("🛡️ VERIPHYS CORE: System Consistency 100%. Listening on {}", addr);
    
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn anchor_content(
    State(state): State<Arc<AppConfig>>,
    mut multipart: Multipart
) -> Result<Json<IntegrityResponse>, String> {
    let mut file_name = String::from("unknown");
    let mut content_data = Vec::new();

    // Parse Multipart safely
    while let Ok(Some(field)) = multipart.next_field().await {
        if field.name() == Some("file") {
            file_name = field.file_name().unwrap_or("unnamed").to_string();
            content_data = field.bytes().await.map_err(|e| e.to_string())?.to_vec();
        }
    }

    if content_data.is_empty() {
        return Err("No file content received".into());
    }

    // 1. SHA3-256 Fingerprinting (FIPS 202)
    let hash_bytes: [u8; 32] = Sha3_256::digest(&content_data).into();
    let file_hash_hex = hex::encode(hash_bytes);

    // 2. Direct On-Chain Anchoring
    let tx_receipt = state.contract
        .anchor_content(hash_bytes) 
        .send()
        .await
        .map_err(|e| format!("Blockchain Submission Error: {}", e))?
        .await
        .map_err(|e| format!("Transaction Confirmation Error: {}", e))?;

    let tx_hash = match tx_receipt {
        Some(receipt) => format!("{:?}", receipt.transaction_hash),
        None => return Err("Transaction failed on-chain".into()),
    };

    // 3. Persistent Local Registry Log
    let log_entry = format!("{},{}\n", file_name, file_hash_hex);
    let mut file = OpenOptions::new()
        .create(true).append(true).open(&state.registry_path).await
        .map_err(|e| e.to_string())?;
    file.write_all(log_entry.as_bytes()).await.map_err(|e| e.to_string())?;

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
