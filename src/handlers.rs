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

// Import shared types from lib.rs
use crate::{AppState, Record, IntegrityResponse};

// Generate blockchain bindings from ABI
abigen!(VeriPhysContract, "./IntegrityLedger.json");

pub async fn anchor_content(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Result<Json<IntegrityResponse>, (StatusCode, String)> {
    
    state.total_requests.fetch_add(1, Ordering::SeqCst);
    let mut file_name = String::from("unknown");
    let mut content_data = Vec::new();

    while let Some(field) = multipart.next_field().await.unwrap_or(None) {
        if field.name() == Some("file") {
            file_name = field.file_name().unwrap_or("unnamed").to_string();
            content_data = field.bytes().await.unwrap_or_default().to_vec();
        }
    }

    if content_data.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "File is empty".into()));
    }

    // SHA3-256 Fingerprinting
    let hash_bytes: [u8; 32] = Sha3_256::digest(&content_data).into();
    let file_hash_hex = hex::encode(hash_bytes);

    // Submit to Smart Contract
    let tx = state.contract.anchor_content(hash_bytes).send().await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    
    let receipt = tx.await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::INTERNAL_SERVER_ERROR, "TX Failed".to_string()))?;

    // Audit Log
    let log = format!("{},{}\n", file_name, file_hash_hex);
    let mut file = tokio::fs::OpenOptions::new().create(true).append(true).open(&state.registry_path).await.unwrap();
    file.write_all(log.as_bytes()).await.ok();

    Ok(Json(IntegrityResponse {
        status: "Success".into(),
        content_hash: file_hash_hex,
        tx_hash: format!("{:?}", receipt.transaction_hash),
        message: "Secured via VeriPhys Protocol".into(),
    }))
}

pub async fn get_stats(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    Json(serde_json::json!({ "total": state.total_requests.load(Ordering::SeqCst) }))
}

pub async fn get_registry(State(state): State<Arc<AppState>>) -> Json<Vec<Record>> {
    let content = tokio::fs::read_to_string(&state.registry_path).await.unwrap_or_default();
    let records = content.lines().filter_map(|l| {
        let p: Vec<&str> = l.split(',').collect();
        if p.len() == 2 { Some(Record { file_name: p[0].into(), file_hash: p[1].into() }) } else { None }
    }).collect();
    Json(records)
}
