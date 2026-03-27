use serde_json::Value;
use uuid::Uuid;

pub struct CreateSchema {
    pub id: Option<Uuid>,
    pub name: String,
    pub schema: Value,
    pub fields_to_mask: Vec<String>,
}
