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

// Generate Rust bindings from the JSON ABI
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

pub async fn anchor_content(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Result<Json<IntegrityResponse>, (StatusCode, String)> {
    state.total_requests.fetch_add(1, Ordering::SeqCst);

    let mut file_name = String::from("unknown");
    let mut content_data = Vec::new();

    // Optimized Multipart extraction for Axum 0.7
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
        return Err((StatusCode::BAD_REQUEST, "File content is missing".into()));
    }

    // 1. SHA3-256 Hashing
    let hash_bytes: [u8; 32] = Sha3_256::digest(&content_data).into();
    let file_hash_hex = hex::encode(hash_bytes);

    // 2. Blockchain submission with conflict detection
    let tx_receipt = state.contract
        .anchor_content(hash_bytes)
        .send()
        .await
        .map_err(|e| {
            if e.to_string().contains("HashAlreadyAnchored") {
                (StatusCode::CONFLICT, "Error: This hash is already secured in the ledger.".into())
            } else {
                (StatusCode::INTERNAL_SERVER_ERROR, format!("Chain Error: {}", e))
            }
        })?
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Mining Error: {}", e)))?
        .ok_or((StatusCode::INTERNAL_SERVER_ERROR, "Transaction failed".to_string()))?;

    let tx_hash = format!("{:?}", tx_receipt.transaction_hash);

    // 3. Local Registry Logging
    let log_entry = format!("{},{}\n", file_name, file_hash_hex);
    let mut file = tokio::fs::OpenOptions::new()
        .create(true).append(true).open(&state.registry_path).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("IO Error: {}", e)))?;
    file.write_all(log_entry.as_bytes()).await.ok();

    Ok(Json(IntegrityResponse {
        status: "Success".into(),
        content_hash: file_hash_hex,
        tx_hash,
        message: "Fingerprint successfully anchored to the blockchain.".into(),
    }))
}

pub async fn get_stats(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    Json(serde_json::json!({
        "total_requests": state.total_requests.load(Ordering::SeqCst),
        "status": "Operational",
        "protocol": "VeriPhys-Core-v1.1"
    }))
}
