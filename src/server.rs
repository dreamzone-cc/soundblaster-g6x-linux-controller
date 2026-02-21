use axum::{
    body::Body,
    extract::{State, Request},
    http::{StatusCode, header, Uri},
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use rust_embed::RustEmbed;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::cors::CorsLayer;

use crate::api::{self, AppState};
use crate::BlasterXG6;

#[derive(RustEmbed)]
#[folder = "frontend/build/"]
pub struct Assets;

pub async fn start_server(device: BlasterXG6) {
    let shared_state = Arc::new(AppState {
        device: Mutex::new(device),
    });

    let app = Router::new()
        .route("/api/status", get(api::get_status))
        .route("/api/feature", post(api::set_feature))
        .route("/api/mixer/status", get(api::get_mixer))
        .route("/api/mixer/feature", post(api::set_mixer))
        .fallback(static_handler)
        .with_state(shared_state)
        .layer(CorsLayer::permissive());

    let addr = SocketAddr::from(([127, 0, 0, 1], 3311));
    println!("Web server listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn static_handler(uri: Uri) -> impl IntoResponse {
    let mut path = uri.path().trim_start_matches('/').to_string();

    if path.is_empty() {
        path = "index.html".to_string();
    }

    match Assets::get(&path) {
        Some(content) => {
            let mime = mime_guess::from_path(&path).first_or_octet_stream();
            ([
                (header::CONTENT_TYPE, mime.as_ref()),
                (header::CACHE_CONTROL, "no-cache, no-store, must-revalidate"),
            ], content.data).into_response()
        }
        None => {
            if path.contains('.') {
                return (StatusCode::NOT_FOUND, "404 Not Found").into_response();
            }
            // Fallback to index.html for SPA routing
             match Assets::get("index.html") {
                Some(content) => {
                    let mime = mime_guess::from_path("index.html").first_or_octet_stream();
                    ([
                        (header::CONTENT_TYPE, mime.as_ref()),
                        (header::CACHE_CONTROL, "no-cache, no-store, must-revalidate"),
                    ], content.data).into_response()
                }
                None => (StatusCode::NOT_FOUND, "404 Not Found").into_response(),
            }
        }
    }
}
