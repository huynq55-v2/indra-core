use axum::{Json, extract::State, http::StatusCode};
use bcrypt::{DEFAULT_COST, hash, verify};
use neo4rs::query;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::AppState; // Cấu trúc chứa Graph connection từ main.rs

#[derive(Deserialize)]
pub struct AuthPayload {
    pub username: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct AuthResponse {
    pub message: String,
    pub user_id: String,
    pub is_genesis: bool,
}

const SYSTEM_ID: &str = "indra_core_genesis_node";

#[derive(Serialize)]
pub struct ErrorResponse {
    pub message: String,
}

// --- API: ĐĂNG KÝ ---
pub async fn register(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<AuthPayload>,
) -> Result<Json<AuthResponse>, (StatusCode, Json<ErrorResponse>)> {
    let hashed_pw = hash(&payload.password, DEFAULT_COST).unwrap();
    let user_id = Uuid::new_v4().to_string();

    // Lưu Node User vào Graph
    let mut txn = state.graph.start_txn().await.unwrap();
    let q = query("CREATE (u:User {id: $id, username: $username, password: $password})")
        .param("id", user_id.clone())
        .param("username", payload.username)
        .param("password", hashed_pw);

    txn.run(q).await.map_err(|err| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                message: format!("Không thể đăng ký. Tên đăng nhập có thể đã tồn tại. Lỗi chi tiết: {:?}", err),
            }),
        )
    })?;
    txn.commit().await.unwrap();

    Ok(Json(AuthResponse {
        message: "Đăng ký thành công".to_string(),
        user_id,
        is_genesis: false,
    }))
}

// --- API: ĐĂNG NHẬP ---
pub async fn login(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<AuthPayload>,
) -> Result<Json<AuthResponse>, (StatusCode, Json<ErrorResponse>)> {
    // 1. Tìm User trong Database
    let q_find =
        query("MATCH (u:User {username: $username}) RETURN u.id AS id, u.password AS password")
            .param("username", payload.username.clone());

    let mut stream = state.graph.execute(q_find).await.unwrap();
    let row = stream
        .next()
        .await
        .unwrap()
        .ok_or((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                message: "Sai tên đăng nhập hoặc mật khẩu.".to_string(),
            }),
        ))?;

    let db_pass: String = row.get("password").unwrap();
    let user_id: String = row.get("id").unwrap();

    // 2. Xác thực mật khẩu
    if !verify(&payload.password, &db_pass).unwrap() {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                message: "Sai tên đăng nhập hoặc mật khẩu.".to_string(),
            }),
        ));
    }

    // 3. Logic: Kiểm tra xem hệ thống đã có Genesis User chưa?
    let q_check_genesis =
        query("MATCH ()-[r:BECOMES_GENESIS]->() RETURN count(r) AS genesis_count");
    let mut gen_stream = state.graph.execute(q_check_genesis).await.unwrap();
    let gen_row = gen_stream.next().await.unwrap().unwrap();
    let genesis_count: i64 = gen_row.get("genesis_count").unwrap();

    // 4. Nếu chưa có ai, phong User này làm Genesis bằng cách tạo Edge
    let is_genesis = if genesis_count == 0 {
        let q_make_genesis = query(
            "
            MATCH (u:User {id: $user_id})
            MERGE (s:System {id: $system_id})
            MERGE (u)-[r:BECOMES_GENESIS]->(s)
        ",
        )
        .param("user_id", user_id.clone())
        .param("system_id", SYSTEM_ID);

        state.graph.run(q_make_genesis).await.unwrap();
        println!(
            "👑 User {} vừa trở thành Genesis User của hệ thống {}!",
            payload.username, SYSTEM_ID
        );
        
        true
    } else {
        // Kiểm tra xem user đăng nhập hiện tại có TRÙNG với Genesis User không
        let q_is_me =
            query("MATCH (u:User {id: $user_id})-[r:BECOMES_GENESIS]->() RETURN count(r) as c")
                .param("user_id", user_id.clone());
        let mut me_stream = state.graph.execute(q_is_me).await.unwrap();
        let me_row = me_stream.next().await.unwrap().unwrap();
        let c: i64 = me_row.get("c").unwrap();
        
        c > 0
    };

    Ok(Json(AuthResponse {
        message: "Đăng nhập thành công".to_string(),
        user_id,
        is_genesis,
    }))
}
