use serde::{Deserialize, Serialize};

// 1. Base Types (Enum)
#[derive(Debug, Serialize, Deserialize)]
pub enum EntityType {
    Node,
    Edge,
}

// 2. Base Traits (Interface)
pub trait GraphEntity {
    fn get_type(&self) -> EntityType;
}

pub trait Edge: GraphEntity {
    fn label(&self) -> &str;
}

// 3. Classes (Structs)
#[derive(Serialize, Deserialize)]
pub struct UserNode {
    pub id: String,
    pub username: String,
    pub password_hash: String,
}

impl GraphEntity for UserNode {
    fn get_type(&self) -> EntityType {
        EntityType::Node
    }
}

#[derive(Serialize, Deserialize)]
pub struct SystemNode {
    pub id: String,
}

impl GraphEntity for SystemNode {
    fn get_type(&self) -> EntityType {
        EntityType::Node
    }
}

// Class Edge: Hành động trở thành Genesis User
#[derive(Serialize, Deserialize)]
pub struct BecomesGenesisEdge {
    pub source_user_id: String,
    pub target_system_id: String,
}

impl GraphEntity for BecomesGenesisEdge {
    fn get_type(&self) -> EntityType {
        EntityType::Edge
    }
}

impl Edge for BecomesGenesisEdge {
    fn label(&self) -> &str {
        "BECOMES_GENESIS"
    }
}

// 4. Các Base Trait cho Đồng thuận (Consensus)
pub trait ConsensusNode: GraphEntity {
    fn get_id(&self) -> &str;
    fn get_status(&self) -> &str;
    fn get_locked_at(&self) -> Option<&String>;
}

// 5. Các Struct cụ thể cho Đồng thuận
#[derive(Serialize, Deserialize)]
pub struct UserRegistrationConsensusNode {
    pub id: String,
    pub username: String,
    pub password_hash: String,
    pub invite_code: String,
    pub status: String,
    pub locked_at: Option<String>,
    pub locked_by: Option<String>,
}

impl GraphEntity for UserRegistrationConsensusNode {
    fn get_type(&self) -> EntityType {
        EntityType::Node
    }
}

impl ConsensusNode for UserRegistrationConsensusNode {
    fn get_id(&self) -> &str {
        &self.id
    }
    fn get_status(&self) -> &str {
        &self.status
    }
    fn get_locked_at(&self) -> Option<&String> {
        self.locked_at.as_ref()
    }
}
