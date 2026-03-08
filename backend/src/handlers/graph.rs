use axum::{extract::{Path, State}, Json, http::StatusCode};
use neo4rs::query;
use serde::Serialize;
use std::sync::Arc;

use crate::AppState;
use crate::handlers::auth::ErrorResponse;

#[derive(Serialize, Clone)]
pub struct NeoNode {
    pub id: String,
    pub label: String,
    pub name: String,
}

#[derive(Serialize, Clone)]
pub struct NeoLink {
    pub source: String,
    pub target: String,
    pub label: String,
}

#[derive(Serialize)]
pub struct GraphDataResponse {
    pub nodes: Vec<NeoNode>,
    pub links: Vec<NeoLink>,
}

pub async fn get_user_graph(
    State(state): State<Arc<AppState>>,
    Path(user_id): Path<String>,
) -> Result<Json<GraphDataResponse>, (StatusCode, Json<ErrorResponse>)> {
    
    // Câu truy vấn này tìm đích danh user, và quét tất cả các mối quan hệ (vô hướng) cách đó 1 hop.
    // Lấy thông tin cả Node và Edge để trả về dạng JSON mạng lưới. Sử dụng startNode(r) và endNode(r)
    // để đảm bảo tính định hướng đúng đắn của mũi tên!
    let cypher = "
        MATCH (u:User {id: $user_id})-[r]-(m)
        WITH r, startNode(r) AS s, endNode(r) AS e
        RETURN DISTINCT
            COALESCE(s.id, s.code) as source_id, [x IN labels(s) WHERE x <> 'Entity'][0] as source_label, s.username as source_name, s.code as source_code,
            COALESCE(e.id, e.code) as target_id, [x IN labels(e) WHERE x <> 'Entity'][0] as target_label, e.username as target_name, e.code as target_code,
            type(r) as rel_type
    ";

    let q = query(cypher).param("user_id", user_id.clone());

    let mut stream = state.graph.execute(q).await.map_err(|err| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                message: format!("Không thể tải đồ thị: {:?}", err),
            }),
        )
    })?;

    let mut nodes = std::collections::HashMap::new();
    let mut links = Vec::new();

    // Luôn luôn nạp ít nhất User hiện tại để dù chưa có kết nối gì vẫn hiển thị điểm trung tâm
    
    // Tuy nhiên chúng ta cần lấy tên user hiện tại trước. Thử truy vấn riêng tên của họ.
    let q_me = query("MATCH (u:User {id: $user_id}) RETURN u.username as username").param("user_id", user_id.clone());
    let mut me_stream = state.graph.execute(q_me).await.unwrap();
    let my_name = if let Ok(Some(me_row)) = me_stream.next().await {
        me_row.get::<String>("username").unwrap_or_else(|_| "Me".to_string())
    } else {
        "Me".to_string()
    };

    nodes.insert(
        user_id.clone(),
        NeoNode {
            id: user_id.clone(),
            label: "User".to_string(),
            name: my_name,
        },
    );

    while let Ok(Some(row)) = stream.next().await {
        let sid: Option<String> = row.get("source_id").unwrap_or(None);
        let slabel: Option<String> = row.get("source_label").unwrap_or(None);
        let sname: Option<String> = row.get("source_name").unwrap_or(None);
        let scode: Option<String> = row.get("source_code").unwrap_or(None);

        let tid: Option<String> = row.get("target_id").unwrap_or(None);
        let tlabel: Option<String> = row.get("target_label").unwrap_or(None);
        let tname: Option<String> = row.get("target_name").unwrap_or(None);
        let tcode: Option<String> = row.get("target_code").unwrap_or(None);

        let rel: Option<String> = row.get("rel_type").unwrap_or(None);

        if let (Some(s_id), Some(s_lab)) = (sid.clone(), slabel) {
            nodes.entry(s_id.clone()).or_insert(NeoNode {
                id: s_id,
                label: s_lab,
                name: sname.unwrap_or_else(|| scode.unwrap_or_else(|| "System".to_string())),
            });
        }

        if let (Some(t_id), Some(t_lab)) = (tid.clone(), tlabel) {
            nodes.entry(t_id.clone()).or_insert(NeoNode {
                id: t_id,
                label: t_lab,
                name: tname.unwrap_or_else(|| tcode.unwrap_or_else(|| "System".to_string())),
            });
        }

        if let (Some(s_id), Some(t_id), Some(r_type)) = (sid, tid, rel) {
            links.push(NeoLink {
                source: s_id,
                target: t_id,
                label: r_type,
            });
        }
    }

    Ok(Json(GraphDataResponse {
        nodes: nodes.into_values().collect(),
        links,
    }))
}
