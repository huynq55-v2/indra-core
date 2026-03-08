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
pub struct PendingRequestInfo {
    pub id: String,
    pub username: String,
    pub invite_code: String,
    pub status: String,
    pub locked_by: Option<String>,
    pub locked_at: Option<String>,
    pub voted_by: Vec<String>, // Danh sách user_id đã vote Approve
    pub total_users: i64,      // Tổng số user trong hệ thống tại lúc fetch
}

#[derive(Serialize)]
pub struct AuthResponse {
    pub message: String,
    pub user_id: Option<String>,
    pub is_genesis: bool,
    pub invite_codes: Vec<InviteCodeInfo>,
    pub request_id: Option<String>,
}

#[derive(Deserialize)]
pub struct VotePayload {
    pub user_id: String,
    pub request_id: String,
    pub approve: bool,
}

#[derive(Deserialize)]
pub struct ConsensusPayload {
    pub user_id: String,
    pub request_id: String,
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

        // Kiểm tra xem mã mời hợp lệ không (nhưng chưa đánh dấu used=true)
        let q_check_code = query(
            "MATCH (inviter:User)-[:GENERATED]->(ic:InviteCode {code: $inviter_code, used: false}) RETURN ic"
        ).param("inviter_code", inviter_code.clone());
        
        let mut check_stream = state.graph.execute(q_check_code).await.map_err(|err| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse { message: format!("Lỗi DB: {:?}", err) }),
            )
        })?;

        if check_stream.next().await.unwrap().is_none() {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    message: "Mã mời này không tồn tại hoặc đã được sử dụng!".to_string(),
                }),
            ));
        }

        let request_id = Uuid::new_v4().to_string();

        let q_create_req = query(
            "MATCH (inviter:User)-[:GENERATED]->(ic:InviteCode {code: $inviter_code, used: false})
             CREATE (req:UserRegistrationConsensusNode:Entity {
                id: $request_id,
                username: $username,
                password: $password,
                invite_code: $inviter_code,
                status: 'PENDING'
            })
            CREATE (inviter)-[:CREATED_CONSENSUS]->(req)
            RETURN req.id"
        )
        .param("request_id", request_id.clone())
        .param("username", payload.username.clone())
        .param("password", hashed_pw.clone())
        .param("inviter_code", inviter_code);

        state.graph.run(q_create_req).await.map_err(|err| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    message: format!("Lỗi tạo request: {:?}", err),
                }),
            )
        })?;

        return Ok(Json(AuthResponse {
            message: "Yêu cầu đăng ký đã được tạo. Vui lòng chờ đồng thuận từ mạng lưới.".to_string(),
            user_id: None,
            is_genesis: false,
            invite_codes: vec![],
            request_id: Some(request_id),
        }));
    } else {
        // 3. Logic dành cho Genesis User (Người đầu tiên)
        let q_genesis = query(
            "CREATE (u:User:Entity {id: $id, username: $username, password: $password})
             MERGE (s:System:Entity {id: $system_id})
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
            user_id: Some(user_id),
            is_genesis: true,
            invite_codes: vec![],
            request_id: None,
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
    let row_opt = stream.next().await.unwrap();
    
    // Nếu không tìm thấy User, kiểm tra xem có đang đợi đồng thuận không
    if row_opt.is_none() {
        let q_pending = query("MATCH (req:UserRegistrationConsensusNode {username: $username}) RETURN req.status AS status")
            .param("username", payload.username.clone());
        let mut stream_pending = state.graph.execute(q_pending).await.unwrap();
        if let Some(_) = stream_pending.next().await.unwrap() {
            return Err((
                StatusCode::FORBIDDEN,
                Json(ErrorResponse {
                    message: "Tài khoản của bạn đang chờ mạng lưới đồng thuận để tham gia vào hệ thống. Vui lòng quay lại sau.".to_string(),
                }),
            ));
        }

        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                message: "Sai tên đăng nhập hoặc mật khẩu.".to_string(),
            }),
        ));
    }

    let row = row_opt.unwrap();

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
        user_id: Some(user_id),
        is_genesis,
        invite_codes: invites,
        request_id: None,
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

// --- API: BỎ PHIẾU ĐỒNG THUẬN ---
pub async fn vote(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<VotePayload>,
) -> Result<Json<AuthResponse>, (StatusCode, Json<ErrorResponse>)> {
    let q_vote = query(
        "MATCH (u:User {id: $user_id}), (req:UserRegistrationConsensusNode {id: $request_id})
         WHERE u.username <> req.username
         MERGE (u)-[v:VOTED_FOR]->(req)
         SET v.approve = $approve, v.updated_at = datetime()
         RETURN req.status AS status"
    )
    .param("user_id", payload.user_id.clone())
    .param("request_id", payload.request_id.clone())
    .param("approve", payload.approve);

    let mut stream = state.graph.execute(q_vote).await.map_err(|err| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse { message: format!("Lỗi DB: {:?}", err) }),
        )
    })?;

    let row = stream.next().await.unwrap().ok_or((
        StatusCode::BAD_REQUEST,
        Json(ErrorResponse { message: "User hoặc Request không tồn tại".to_string() }),
    ))?;

    let status: String = row.get("status").unwrap_or_else(|_| "PENDING".to_string());

    if status != "PENDING" {
         return Err((StatusCode::BAD_REQUEST, Json(ErrorResponse { message: "Request không ở trạng thái PENDING".to_string() })));
    }

    let q_check = query(
        "MATCH (u:User) WITH count(u) AS total_users
         MATCH (req:UserRegistrationConsensusNode {id: $request_id})
         OPTIONAL MATCH (req)<-[v:VOTED_FOR {approve: true}]-(:User)
         WITH total_users, req, count(v) AS total_approves
         RETURN total_users, total_approves, req.invite_code AS invite_code, req.username AS username, req.password AS password"
    ).param("request_id", payload.request_id.clone());

    let mut check_stream = state.graph.execute(q_check).await.unwrap();
    let check_row = check_stream.next().await.unwrap().unwrap();
    let total_users: i64 = check_row.get("total_users").unwrap();
    let total_approves: i64 = check_row.get("total_approves").unwrap();

    // AUTO-APPROVE LOGIC (1 PHASE)
    if total_approves * 2 > total_users {
        let invite_code: String = check_row.get("invite_code").unwrap();
        let username: String = check_row.get("username").unwrap();
        let password: String = check_row.get("password").unwrap();

        let new_user_id = Uuid::new_v4().to_string();

        let q_finalize = query(
            "MATCH (req:UserRegistrationConsensusNode {id: $request_id})
             MATCH (inviter:User)-[:GENERATED]->(ic:InviteCode {code: $invite_code, used: false})
             SET ic.used = true
             CREATE (u:User:Entity {id: $new_user_id, username: $username, password: $password})
             CREATE (u)-[:INVITED_BY]->(inviter)
             CREATE (u)-[:USED_CODE]->(ic)
             CREATE (req)-[:RESULTS_IN]->(u)
             SET req.status = 'APPROVED'"
        )
        .param("request_id", payload.request_id.clone())
        .param("invite_code", invite_code)
        .param("username", username)
        .param("password", password)
        .param("new_user_id", new_user_id.clone());

        state.graph.run(q_finalize).await.unwrap();

        return Ok(Json(AuthResponse {
            message: "Đồng thuận đạt > 50%. Tài khoản đã được duyệt thành công!".to_string(),
            user_id: Some(new_user_id),
            is_genesis: false,
            invite_codes: vec![],
            request_id: None,
        }));
    }

    Ok(Json(AuthResponse {
        message: "Cập nhật phiếu bầu thành công".to_string(),
        user_id: Some(payload.user_id),
        is_genesis: false,
        invite_codes: vec![],
        request_id: Some(payload.request_id),
    }))
}

// --- API: LẤY DANH SÁCH YÊU CẦU ĐANG CHỜ (ĐỂ HIỂN THỊ UI) ---
pub async fn get_pending_requests(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<PendingRequestInfo>>, (StatusCode, Json<ErrorResponse>)> {
    let q = query(
        "MATCH (u_all:User) WITH count(u_all) AS total_users
         MATCH (req:UserRegistrationConsensusNode)
         WHERE req.status = 'PENDING'
         OPTIONAL MATCH (u:User)-[v:VOTED_FOR {approve: true}]->(req)
         RETURN total_users, req.id AS id, req.username AS username, req.invite_code AS invite_code, 
                req.status AS status, req.locked_by AS locked_by, req.locked_at AS locked_at,
                collect(u.id) AS voted_by
         ORDER BY req.locked_at DESC"
    );

    let mut stream = state.graph.execute(q).await.map_err(|err| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse { message: format!("Lỗi DB: {:?}", err) }),
        )
    })?;

    let mut list = vec![];
    while let Ok(Some(row)) = stream.next().await {
        let id: String = row.get("id").unwrap();
        let username: String = row.get("username").unwrap();
        let invite_code: String = row.get("invite_code").unwrap();
        let status: String = row.get("status").unwrap();
        let total_users: i64 = row.get("total_users").unwrap_or(0);
        
        let mut locked_by_opt = None;
        if let Ok(lb) = row.get::<String>("locked_by") {
            locked_by_opt = Some(lb);
        }
        
        let mut locked_at_opt = None;
        if let Ok(la) = row.get::<String>("locked_at") {
            locked_at_opt = Some(la.to_string());
        }

        let voted_by: Vec<String> = row.get("voted_by").unwrap_or_else(|_| vec![]);

        list.push(PendingRequestInfo {
            id,
            username,
            invite_code,
            status,
            locked_by: locked_by_opt,
            locked_at: locked_at_opt,
            voted_by,
            total_users,
        });
    }

    Ok(Json(list))
}
