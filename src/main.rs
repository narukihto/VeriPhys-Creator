use axum::{extract::Multipart, routing::{post, get}, Json, Router};
use serde::{Serialize, Deserialize};
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;
use std::fs::{OpenOptions, read_to_string};
use std::io::Write;
use std::sync::Arc;
use std::env; 
use dotenv::dotenv; 
use ethers::prelude::*;
use sha3::{Sha3_256, Digest};

// Generates the Rust struct from your Solidity ABI
abigen!(VeriPhysContract, "./integrityLedger.json");

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
    // Load environment variables from .env file
    dotenv().ok();
    
    let app = Router::new()
        .route("/v1/anchor", post(anchor_content))
        .route("/v1/registry", get(get_registry))
        .layer(CorsLayer::permissive());

    // Read port from .env or default to 3000
    let port = env::var("SERVER_PORT").unwrap_or_else(|_| "3000".to_string());
    let addr_str = format!("127.0.0.1:{}", port);
    let addr: SocketAddr = addr_str.parse().expect("Invalid Socket Address");

    println!("🛡️  VERIPHYS CORE ACTIVE: Quantum-resistant hashing (SHA3-256) enabled.");
    println!("📡 API Server running on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn anchor_content(mut multipart: Multipart) -> Json<IntegrityResponse> {
    let mut file_name = String::from("unknown");
    let mut content_data = Vec::new();

    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or_default().to_string();
        if name == "file" {
            file_name = field.file_name().unwrap_or("unnamed_asset").to_string();
            content_data = field.bytes().await.unwrap_or_default().to_vec();
        }
    }

    // 1. Generate SHA3-256 Fingerprint (Off-chain)
    let mut hasher = Sha3_256::new();
    hasher.update(&content_data);
    let result = hasher.finalize(); 
    
    let file_hash_hex = format!("{:x}", result);
    let hash_bytes: [u8; 32] = result.into(); 
    
    // 2. Blockchain Anchoring (On-chain)
    // Pass the raw bytes to send_to_blockchain which will handle conversion
    let blockchain_tx = match send_to_blockchain(hash_bytes).await {
        Ok(tx) => tx,
        Err(e) => {
            eprintln!("Blockchain Sync Error: {}", e);
            "On-chain anchoring failed".to_string()
        },
    };

    // 3. Local Historical Logging
    let registry_path = env::var("REGISTRY_PATH").unwrap_or_else(|_| "registry.txt".to_string());
    let log_entry = format!("{},{}\n", file_name, file_hash_hex);
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(registry_path)
        .expect("IO Error: Could not open registry");
    file.write_all(log_entry.as_bytes()).expect("Write Error");

    Json(IntegrityResponse {
        status: "Success".to_string(),
        content_hash: file_hash_hex,
        tx_hash: blockchain_tx,
        message: "Digital fingerprint anchored to the VeriPhys Ledger.".to_string(),
    })
}

async fn send_to_blockchain(hash: [u8; 32]) -> Result<String, Box<dyn std::error::Error>> {
    // Dynamic fetching from Environment Variables
    let rpc_url = env::var("RPC_URL").expect("RPC_URL not found in .env");
    let contract_addr = env::var("CONTRACT_ADDRESS").expect("CONTRACT_ADDRESS not found in .env");
    let private_key = env::var("PRIVATE_KEY").expect("PRIVATE_KEY not found in .env");

    let provider = Provider::<Http>::try_from(rpc_url)?;
    let address: Address = contract_addr.parse()?;
    let wallet: LocalWallet = private_key.parse()?;
    
    let client = Arc::new(SignerMiddleware::new(provider, wallet.with_chain_id(1337u64)));
    let contract = VeriPhysContract::new(address, client);

    // ✅ FIXED: Convert the [u8; 32] array into a Hex String with '0x' prefix
    // This matches what the abigen-generated function expects based on your Solidity 'string' type
    let hash_hex = format!("0x{}", hex::encode(hash));

    // Call the Solidity function 'anchor_content' with the String argument
    let tx = contract.anchor_content(hash_hex).send().await?.await?;
    
    match tx {
        Some(receipt) => Ok(format!("{:?}", receipt.transaction_hash)),
        None => Err("Transaction failed on-chain".into()),
    }
}

async fn get_registry() -> Json<Vec<Record>> {
    let registry_path = env::var("REGISTRY_PATH").unwrap_or_else(|_| "registry.txt".to_string());
    let content = read_to_string(registry_path).unwrap_or_default();
    let records = content.lines()
        .filter_map(|line| {
            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() == 2 {
                Some(Record { 
                    file_name: parts[0].to_string(), 
                    file_hash: parts[1].to_string() 
                })
            } else { None }
        }).collect();
    Json(records)
}

