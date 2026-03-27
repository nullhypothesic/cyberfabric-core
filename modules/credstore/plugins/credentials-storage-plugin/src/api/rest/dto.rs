use chrono::{DateTime, Utc};
use serde_json::Value;
use uuid::Uuid;

use crate::domain::credential::Credential;
use crate::infra::db::entity::{credential_definition, schema};
use crate::infra::db::repo::credential_definitions::UpdateCredentialDefinition;
use crate::infra::db::repo::schemas::UpdateSchema;

// ---------------------------------------------------------------------------
// Schema DTOs
// ---------------------------------------------------------------------------

#[derive(Debug)]
#[modkit_macros::api_dto(response)]
pub struct SchemaDto {
    #[schema(value_type = String)]
    pub id: Uuid,
    pub name: String,
    #[schema(value_type = String)]
    pub created: DateTime<Utc>,
    pub schema: Value,
    pub fields_to_mask: Vec<String>,
    #[schema(value_type = String)]
    pub application_id: Uuid,
}

impl From<schema::Model> for SchemaDto {
    fn from(m: schema::Model) -> Self {
        Self {
            id: m.id,
            name: m.name,
            created: m.created,
            schema: m.schema,
            fields_to_mask: m.fields_to_mask,
            application_id: m.application_id,
        }
    }
}

#[derive(Debug)]
#[modkit_macros::api_dto(request)]
pub struct CreateSchemaRequest {
    #[serde(default)]
    #[schema(value_type = Option<String>)]
    pub id: Option<Uuid>,
    pub name: String,
    pub schema: Value,
    #[serde(default)]
    pub fields_to_mask: Vec<String>,
}

#[derive(Debug)]
#[modkit_macros::api_dto(request)]
pub struct UpdateSchemaRequest {
    pub name: String,
    pub schema: Value,
    #[serde(default)]
    pub fields_to_mask: Vec<String>,
}

impl From<UpdateSchemaRequest> for UpdateSchema {
    fn from(req: UpdateSchemaRequest) -> Self {
        Self {
            name: req.name,
            schema: req.schema,
            fields_to_mask: req.fields_to_mask,
        }
    }
}

// ---------------------------------------------------------------------------
// Credential Definition DTOs
// ---------------------------------------------------------------------------

#[derive(Debug)]
#[modkit_macros::api_dto(response)]
pub struct CredentialDefinitionDto {
    #[schema(value_type = String)]
    pub id: Uuid,
    pub name: String,
    pub description: String,
    #[schema(value_type = String)]
    pub schema_id: Uuid,
    #[schema(value_type = String)]
    pub created: DateTime<Utc>,
    pub default_value: Value,
    #[schema(value_type = Vec<String>)]
    pub allowed_app_ids: Vec<Uuid>,
    #[schema(value_type = String)]
    pub application_id: Uuid,
}

impl From<credential_definition::Model> for CredentialDefinitionDto {
    fn from(m: credential_definition::Model) -> Self {
        Self {
            id: m.id,
            name: m.name,
            description: m.description,
            schema_id: m.schema_id,
            created: m.created,
            default_value: m.default_value,
            allowed_app_ids: m.allowed_app_ids,
            application_id: m.application_id,
        }
    }
}

#[derive(Debug)]
#[modkit_macros::api_dto(request)]
pub struct CreateCredentialDefinitionRequest {
    #[serde(default)]
    #[schema(value_type = Option<String>)]
    pub id: Option<Uuid>,
    pub name: String,
    pub description: String,
    #[schema(value_type = String)]
    pub schema_id: Uuid,
    pub default_value: Value,
    #[serde(default)]
    #[schema(value_type = Option<Vec<String>>)]
    pub allowed_app_ids: Option<Vec<Uuid>>,
}

#[derive(Debug)]
#[modkit_macros::api_dto(request)]
pub struct UpdateCredentialDefinitionRequest {
    pub name: String,
    pub description: String,
    pub default_value: Value,
    #[serde(default)]
    #[schema(value_type = Option<Vec<String>>)]
    pub allowed_app_ids: Option<Vec<Uuid>>,
}

impl From<UpdateCredentialDefinitionRequest> for UpdateCredentialDefinition {
    fn from(req: UpdateCredentialDefinitionRequest) -> Self {
        Self {
            name: req.name,
            description: req.description,
            default_value: req.default_value,
            allowed_app_ids: req.allowed_app_ids.unwrap_or_default(),
        }
    }
}

// ---------------------------------------------------------------------------
// Credential DTOs
// ---------------------------------------------------------------------------

#[derive(Debug)]
#[modkit_macros::api_dto(response)]
pub struct CredentialDto {
    #[schema(value_type = String)]
    pub id: Uuid,
    pub definition_name: Option<String>,
    #[schema(value_type = String)]
    pub created: DateTime<Utc>,
    pub value: Value,
    pub propagate: bool,
}

impl From<Credential> for CredentialDto {
    fn from(c: Credential) -> Self {
        Self {
            id: c.id,
            definition_name: c.definition_name,
            created: c.created,
            value: c.value,
            propagate: c.propagate,
        }
    }
}

#[derive(Debug)]
#[modkit_macros::api_dto(request)]
pub struct CreateCredentialRequest {
    #[serde(default)]
    #[schema(value_type = Option<String>)]
    pub id: Option<Uuid>,
    pub definition_name: String,
    pub value: Value,
    pub propagate: bool,
}

#[derive(Debug)]
#[modkit_macros::api_dto(request)]
pub struct UpdateCredentialRequest {
    pub value: Value,
    pub propagate: bool,
}
