use axum::{
    extract::{Multipart, State},
    Json,
    response::IntoResponse,
    http::StatusCode,
};
use std::sync::Arc;
use std::sync::atomic::Ordering;
use ethers::prelude::*;
use sha3::{Sha3_256, Digest};
use tokio::io::AsyncWriteExt;

use super::{AppState, Record, IntegrityResponse};

// Generate blockchain bindings from ABI
abigen!(VeriPhysContract, "./IntegrityLedger.json");

pub async fn anchor_content(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Result<Json<IntegrityResponse>, (StatusCode, String)> {
    
    // Increment request counter
    state.total_requests.fetch_add(1, Ordering::SeqCst);
    
    let mut file_name = String::from("unknown");
    let mut content_data = Vec::new();

    // Parse multipart form data
    while let Some(field) = multipart.next_field().await.unwrap_or(None) {
        if field.name() == Some("file") {
            file_name = field.file_name().unwrap_or("unnamed").to_string();
            content_data = field.bytes().await.unwrap_or_default().to_vec();
        }
    }

    if content_data.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "File buffer is empty".into()));
    }

    // Generate SHA3-256 Fingerprint
    let hash_bytes: [u8; 32] = Sha3_256::digest(&content_data).into();
    let file_hash_hex = hex::encode(hash_bytes);

    // --- BLOCKCHAIN INTERACTION (Fixed for E0716) ---
    
    // 1. Create a binding for the contract call to extend its lifetime
    let contract_call = state.contract.anchor_content(hash_bytes);

    // 2. Submit transaction and get the pending transaction object
    let pending_tx = contract_call
        .send()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Blockchain Send Error: {}", e)))?;
    
    // 3. Await the receipt (wait for the block to be mined)
    let receipt = pending_tx
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Mining Confirmation Error: {}", e)))?
        .ok_or((StatusCode::INTERNAL_SERVER_ERROR, "Transaction was dropped from mempool".to_string()))?;

    // --- END OF BLOCKCHAIN INTERACTION ---

    // Audit Log: Update local registry file
    let log_entry = format!("{},{}\n", file_name, file_hash_hex);
    let mut file = tokio::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&state.registry_path)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Registry I/O Error: {}", e)))?;
        
    file.write_all(log_entry.as_bytes()).await.ok();

    // Return success response
    Ok(Json(IntegrityResponse {
        status: "Success".into(),
        content_hash: file_hash_hex,
        tx_hash: format!("{:?}", receipt.transaction_hash),
        message: "Content secured via VeriPhys Protocol".into(),
    }))
}

pub async fn get_stats(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    Json(serde_json::json!({ 
        "total_requests": state.total_requests.load(Ordering::SeqCst) 
    }))
}

pub async fn get_registry(State(state): State<Arc<AppState>>) -> Json<Vec<Record>> {
    let content = tokio::fs::read_to_string(&state.registry_path).await.unwrap_or_default();
    let records = content.lines().filter_map(|line| {
        let parts: Vec<&str> = line.split(',').collect();
        if parts.len() == 2 { 
            Some(Record { 
                file_name: parts[0].into(), 
                file_hash: parts[1].into() 
            }) 
        } else { 
            None 
        }
    }).collect();
    Json(records)
}
