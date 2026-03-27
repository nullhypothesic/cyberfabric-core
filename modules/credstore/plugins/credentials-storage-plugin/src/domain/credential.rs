use chrono::{DateTime, Utc};
use serde_json::Value;
use uuid::Uuid;

use crate::infra::db::entity::credential;

pub struct CreateCredential {
    pub id: Option<Uuid>,
    pub definition_name: String,
    pub value: Value,
    pub propagate: bool,
}

pub struct UpdateCredential {
    pub value: Value,
    pub propagate: bool,
    pub tenant_id: Uuid,
    pub definition_name: String,
}

#[derive(Debug, Clone)]
pub struct Credential {
    pub id: Uuid,
    pub definition_name: Option<String>,
    pub created: DateTime<Utc>,
    pub value: Value, // masked value
    pub propagate: bool,
}

impl Credential {
    pub fn add_definition_name(mut self, name: String) -> Self {
        self.definition_name = Some(name);
        self
    }
}

impl From<credential::Model> for Credential {
    fn from(m: credential::Model) -> Self {
        Self {
            id: m.id,
            definition_name: None,
            created: m.created,
            value: m.masked_value,
            propagate: m.propagate,
        }
    }
}
