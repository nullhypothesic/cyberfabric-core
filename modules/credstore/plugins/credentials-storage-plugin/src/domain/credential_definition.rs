use serde_json::Value;
use uuid::Uuid;

pub struct CreateCredentialDefinition {
    pub id: Option<Uuid>,
    pub name: String,
    pub description: String,
    pub schema_id: Uuid,
    pub default_value: Value,
    pub allowed_app_ids: Vec<Uuid>,
}
