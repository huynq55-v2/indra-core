mod handlers;
mod models;

use axum::{routing::post, Router};
use neo4rs::{ConfigBuilder, Graph};
use std::sync::Arc;
use tower_http::cors::CorsLayer;

// Định nghĩa trạng thái dùng chung (Shared State) để mọi API có thể truy cập DB
struct AppState {
    graph: Graph,
}

#[tokio::main]
async fn main() {
    // 1. Cấu hình kết nối Neo4j (Thông tin từ docker-compose)
    let uri = "127.0.0.1:7687";
    let user = "neo4j";
    let pass = "indracore123";

    let config = ConfigBuilder::default()
        .uri(uri)
        .user(user)
        .password(pass)
        .build()
        .unwrap();

    // 2. Kết nối vào Graph Database
    let graph = Graph::connect(config)
        .await
        .expect("Không thể kết nối Neo4j! Hãy chắc chắn Docker đã chạy.");
    let shared_state = Arc::new(AppState { graph });

    println!("🚀 IndraCore Backend đang chạy tại http://0.0.0.0:3000");
    println!("📊 Đã kết nối thành công tới Neo4j tại {}", uri);

    // 3. Định nghĩa các Route (Đường dẫn API)
    let app = Router::new()
        .route("/api/auth/register", post(handlers::auth::register))
        .route("/api/auth/login", post(handlers::auth::login))
        .layer(CorsLayer::permissive())
        .with_state(shared_state);

    // 4. Chạy Server
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
