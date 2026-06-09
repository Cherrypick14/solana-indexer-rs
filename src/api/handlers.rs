use axum::{
    extract::{Path, State, Query},
    Json,
    http::StatusCode,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use crate::api::AppState;

#[derive(Deserialize, Debug)]
pub struct TransactionQuery {
    pub account: Option<String>,
    pub program_id: Option<String>,
    pub start_slot: Option<u64>,
    pub end_slot: Option<u64>,
    pub page: Option<u32>,
    pub limit: Option<u32>,
}

#[derive(Serialize)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub page: u32,
    pub limit: u32,
    pub total: i64,
}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

pub async fn get_transaction(
    Path(signature): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    match state.db.get_transaction(&signature).await {
        Ok(Some(tx)) => (StatusCode::OK, Json(tx)).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, Json(ErrorResponse { error: "Transaction not found".into() })).into_response(),
        Err(e) => {
            tracing::error!("Failed to fetch transaction: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: "Internal server error".into() })).into_response()
        }
    }
}

pub async fn get_transactions(
    Query(query): Query<TransactionQuery>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    match state.db.get_transactions(&query).await {
        Ok((data, total)) => {
            let limit = query.limit.unwrap_or(20);
            let page = query.page.unwrap_or(1);
            let response = PaginatedResponse {
                data,
                page,
                limit,
                total,
            };
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(e) => {
            tracing::error!("Failed to fetch transactions: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: "Internal server error".into() })).into_response()
        }
    }
}
