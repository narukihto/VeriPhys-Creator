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

// Generate Rust bindings for the Blockchain Contract
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

/// 1. BLOCKCHAIN ANCHORING HANDLER
/// Receives a file, computes SHA3-256, and submits to the Ledger.
pub async fn anchor_content(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Result<Json<IntegrityResponse>, (StatusCode, String)> {
    // Increment global request counter
    state.total_requests.fetch_add(1, Ordering::SeqCst);

    let mut file_name = String::from("unknown");
    let mut content_data = Vec::new();

    // Secure Multipart Parsing
    while let Ok(Some(field)) = multipart.next_field().await {
        if field.name() == Some("file") {
            file_name = field.file_name().unwrap_or("unnamed").to_string();
            content_data = field.bytes().await
                .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?
                .to_vec();
        }
    }

    if content_data.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "No file content detected".into()));
    }

    // A. SHA3-256 Fingerprinting (The "Digital Physics" layer)
    let hash_bytes: [u8; 32] = Sha3_256::digest(&content_data).into();
    let file_hash_hex = hex::encode(hash_bytes);

    // B. Blockchain Anchoring (The "Immutable Ledger" layer)
    let tx_receipt = state.contract
        .anchor_content(hash_bytes)
        .send()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Blockchain Error: {}", e)))?
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Confirmation Error: {}", e)))?
        .ok_or((StatusCode::INTERNAL_SERVER_ERROR, "Transaction failed to anchor".to_string()))?;

    let tx_hash = format!("{:?}", tx_receipt.transaction_hash);

    // C. Local Registry Logging (Async File I/O)
    let log_entry = format!("{},{}\n", file_name, file_hash_hex);
    let mut file = tokio::fs::OpenOptions::new()
        .create(true).append(true).open(&state.registry_path).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    file.write_all(log_entry.as_bytes()).await.ok();

    Ok(Json(IntegrityResponse {
        status: "Success".into(),
        content_hash: file_hash_hex,
        tx_hash,
        message: "Digital fingerprint secured in the VeriPhys Ledger.".into(),
    }))
}

/// 2. REGISTRY API
/// Returns the list of all anchored files from the local record.
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

/// 3. SECURITY STATS API
/// Monitoring real-time system performance.
pub async fn get_stats(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    Json(serde_json::json!({
        "total_requests": state.total_requests.load(Ordering::SeqCst),
        "registry_path": state.registry_path,
    }))
}
