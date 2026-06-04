pub mod handlers;

#[cfg(test)]
pub mod tests;

use axum::{routing::get, Router};
use std::sync::Arc;
use crate::database::Database;
use tower_http::trace::TraceLayer;
use tower_http::cors::CorsLayer;

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Database>,
}

pub fn create_router(db: Arc<Database>) -> Router {
    let state = AppState { db };

    Router::new()
        .route("/transactions", get(handlers::get_transactions))
        .route("/transactions/:signature", get(handlers::get_transaction))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
