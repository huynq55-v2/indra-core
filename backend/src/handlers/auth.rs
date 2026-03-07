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
    pub invite_code: Option<String>,
}

#[derive(Serialize)]
pub struct InviteCodeInfo {
    pub code: String,
    pub used: bool,
}

#[derive(Serialize)]
pub struct AuthResponse {
    pub message: String,
    pub user_id: String,
    pub is_genesis: bool,
    pub invite_codes: Vec<InviteCodeInfo>,
}

#[derive(Deserialize)]
pub struct GenerateInvitePayload {
    pub user_id: String,
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

    // 1. Kiểm tra xem hệ thống đã có User nào chưa
    let q_check_users = query("MATCH (u:User) RETURN count(u) AS user_count");
    let mut count_stream = state.graph.execute(q_check_users).await.unwrap();
    let count_row = count_stream.next().await.unwrap().unwrap();
    let user_count: i64 = count_row.get("user_count").unwrap();

    let is_genesis = user_count == 0;

    // 2. Xử lý Logic Mã Mời BẮT BUỘC nếu KHÔNG PHẢI Genesis
    if !is_genesis {
        let inviter_code = payload.invite_code.clone().unwrap_or_default();
        if inviter_code.is_empty() {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    message: "Mã mời là bắt buộc để tham gia mạng lưới!".to_string(),
                }),
            ));
        }

        // Kiểm tra xem mã có tồn tại và chưa được dùng không
        let q_create = query(
            "MATCH (inviter:User)-[:GENERATED]->(ic:InviteCode {code: $inviter_code, used: false})
             SET ic.used = true
             CREATE (u:User {id: $id, username: $username, password: $password})
             CREATE (u)-[:INVITED_BY]->(inviter)
             CREATE (u)-[:USED_CODE]->(ic)
             RETURN u.id",
        )
        .param("inviter_code", inviter_code)
        .param("id", user_id.clone())
        .param("username", payload.username.clone())
        .param("password", hashed_pw.clone());

        let mut stream = state.graph.execute(q_create).await.map_err(|err| {
            (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    message: format!("Lỗi hệ thống khi đăng ký bằng mã: {:?}", err),
                }),
            )
        })?;

        // Nếu không return row nào tức là mã mời sai hoặc đã bị sử dụng
        if stream.next().await.unwrap().is_none() {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    message: "Mã mời này không tồn tại hoặc đã được sử dụng!".to_string(),
                }),
            ));
        }

        return Ok(Json(AuthResponse {
            message: "Đăng ký thành công".to_string(),
            user_id,
            is_genesis: false,
            invite_codes: vec![],
        }));
    } else {
        // 3. Logic dành cho Genesis User (Người đầu tiên)
        let q_genesis = query(
            "CREATE (u:User {id: $id, username: $username, password: $password})
             MERGE (s:System {id: $system_id})
             CREATE (u)-[:BECOMES_GENESIS]->(s)",
        )
        .param("id", user_id.clone())
        .param("username", payload.username.clone())
        .param("password", hashed_pw)
        .param("system_id", SYSTEM_ID);

        state.graph.run(q_genesis).await.map_err(|err| {
            (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    message: format!("Không thể tạo Genesis User. Lỗi chi tiết: {:?}", err),
                }),
            )
        })?;

        println!(
            "👑 User {} vừa trở thành Genesis User của hệ thống {}!",
            payload.username, SYSTEM_ID
        );

        return Ok(Json(AuthResponse {
            message: "Đăng ký thành công - Bạn là Genesis Node!".to_string(),
            user_id,
            is_genesis: true,
            invite_codes: vec![],
        }));
    }
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
    let row = stream.next().await.unwrap().ok_or((
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

    // 3. Lấy danh sách mã mời mà user này đã tạo
    let q_invites = query("MATCH (u:User {id: $id})-[:GENERATED]->(ic:InviteCode) RETURN ic.code AS code, ic.used AS used")
        .param("id", user_id.clone());

    let mut invites = vec![];
    let mut inv_stream = state.graph.execute(q_invites).await.unwrap();
    while let Ok(Some(inv_row)) = inv_stream.next().await {
        let code: String = inv_row.get("code").unwrap();
        let used: bool = inv_row.get("used").unwrap();
        invites.push(InviteCodeInfo { code, used });
    }

    // 4. Kiểm tra xem user đăng nhập hiện tại có phải là Genesis User không
    let q_is_me =
        query("MATCH (u:User {id: $user_id})-[r:BECOMES_GENESIS]->() RETURN count(r) as c")
            .param("user_id", user_id.clone());
    let mut me_stream = state.graph.execute(q_is_me).await.unwrap();
    let me_row = me_stream.next().await.unwrap().unwrap();
    let c: i64 = me_row.get("c").unwrap();

    let is_genesis = c > 0;

    Ok(Json(AuthResponse {
        message: "Đăng nhập thành công".to_string(),
        user_id,
        is_genesis,
        invite_codes: invites,
    }))
}

// --- API: TẠO MÃ MỜI MỚI ---
pub async fn generate_invite(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<GenerateInvitePayload>,
) -> Result<Json<InviteCodeInfo>, (StatusCode, Json<ErrorResponse>)> {
    let new_code_str = format!("INA-{}", &Uuid::new_v4().to_string()[0..36]).to_uppercase();

    // Check if user exists
    let q_create = query(
        "MATCH (u:User {id: $user_id})
         CREATE (ic:InviteCode {code: $code, used: false})
         CREATE (u)-[:GENERATED]->(ic)
         RETURN ic.code AS code, ic.used AS used",
    )
    .param("user_id", payload.user_id.clone())
    .param("code", new_code_str);

    let mut stream = state.graph.execute(q_create).await.map_err(|err| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                message: format!("Không thể tạo mã mời: {:?}", err),
            }),
        )
    })?;

    if let Some(row) = stream.next().await.unwrap() {
        let code: String = row.get("code").unwrap();
        let used: bool = row.get("used").unwrap();
        return Ok(Json(InviteCodeInfo { code, used }));
    }

    Err((
        StatusCode::BAD_REQUEST,
        Json(ErrorResponse {
            message: "User không tồn tại.".to_string(),
        }),
    ))
}
