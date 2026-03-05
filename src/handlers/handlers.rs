use axum::{
    extract::{Multipart, State},
    Json,
    response::IntoResponse,
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::atomic::Ordering;
use crate::AppState; 
use ethers::prelude::*;
use sha3::{Sha3_256, Digest};
use tokio::io::AsyncWriteExt;

// Generate Rust bindings from the Solidity ABI JSON
// Ensure IntegrityLedger.json is in your project root
abigen!(VeriPhysContract, "./IntegrityLedger.json");

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

/// 1. ANCHOR CONTENT HANDLER
/// Receives file, computes SHA3-256 fingerprint, and submits to the blockchain.
pub async fn anchor_content(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Result<Json<IntegrityResponse>, (StatusCode, String)> {
    // Increment global request counter for monitoring
    state.total_requests.fetch_add(1, Ordering::SeqCst);

    let mut file_name = String::from("unknown");
    let mut content_data = Vec::new();

    // Secure Multipart Extraction (Axum 0.7 compatible)
    while let Some(field) = multipart.next_field().await
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Multipart Error: {}", e)))? 
    {
        if field.name() == Some("file") {
            file_name = field.file_name().unwrap_or("unnamed").to_string();
            content_data = field.bytes().await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
                .to_vec();
        }
    }

    if content_data.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "Zero-byte files cannot be anchored".into()));
    }

    // A. Cryptographic Fingerprinting (SHA3-256)
    let hash_bytes: [u8; 32] = Sha3_256::digest(&content_data).into();
    let file_hash_hex = hex::encode(hash_bytes);

    // B. Blockchain Anchoring with Solidity Custom Error detection
    let tx_receipt = state.contract
        .anchor_content(hash_bytes)
        .send()
        .await
        .map_err(|e| {
            // Check for Solidity Custom Error: HashAlreadyAnchored
            if e.to_string().contains("HashAlreadyAnchored") {
                (StatusCode::CONFLICT, "Security Alert: This file hash is already registered in the ledger.".into())
            } else {
                (StatusCode::INTERNAL_SERVER_ERROR, format!("Blockchain Node Error: {}", e))
            }
        })?
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Transaction Confirmation Error: {}", e)))?
        .ok_or((StatusCode::INTERNAL_SERVER_ERROR, "Transaction was dropped by the network".to_string()))?;

    let tx_hash = format!("{:?}", tx_receipt.transaction_hash);

    // C. Local Audit Trail (Async File I/O)
    let log_entry = format!("{},{}\n", file_name, file_hash_hex);
    let mut file = tokio::fs::OpenOptions::new()
        .create(true).append(true).open(&state.registry_path).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("File System Error: {}", e)))?;
    file.write_all(log_entry.as_bytes()).await.ok();

    Ok(Json(IntegrityResponse {
        status: "Success".into(),
        content_hash: file_hash_hex,
        tx_hash,
        message: "Digital fingerprint immutable and secured via VeriPhys Protocol.".into(),
    }))
}

/// 2. STATS API
/// Returns real-time health and usage metrics.
pub async fn get_stats(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    Json(serde_json::json!({
        "total_anchors": state.total_requests.load(Ordering::SeqCst),
        "status": "Operational",
        "hashing_algorithm": "SHA3-256 (FIPS 202)",
        "engine_version": "1.1.0-stable"
    }))
}

/// 3. REGISTRY API
/// Retrieves history of all locally anchored assets.
pub async fn get_registry(State(state): State<Arc<AppState>>) -> Json<Vec<Record>> {
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
