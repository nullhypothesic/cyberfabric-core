use std::collections::HashSet;
use std::sync::Arc;

use modkit_db::{DBProvider, DbError};
use serde_json::Value;
use uuid::Uuid;

use chrono::Utc;

use crate::domain::error::ServiceError;
use crate::domain::schema::CreateSchema;
use crate::infra::db::entity::schema;
use crate::infra::db::repo::schemas::{SchemasRepo, UpdateSchema};

pub struct SchemasService {
    db: Arc<DBProvider<DbError>>,
    application_id: Uuid,
    repo: SchemasRepo,
}

impl SchemasService {
    pub fn new(db: Arc<DBProvider<DbError>>, application_id: Uuid) -> Self {
        Self {
            db,
            application_id,
            repo: SchemasRepo,
        }
    }

    pub async fn list(&self) -> Result<Vec<schema::Model>, ServiceError> {
        let conn = self
            .db
            .conn()
            .map_err(|e| ServiceError::Internal(e.to_string()))?;
        self.repo.find_all(&conn, None).await.map_err(Into::into)
    }

    pub async fn get(&self, id: Uuid) -> Result<schema::Model, ServiceError> {
        let conn = self
            .db
            .conn()
            .map_err(|e| ServiceError::Internal(e.to_string()))?;
        self.repo
            .find_by_id(&conn, id)
            .await?
            .ok_or_else(|| ServiceError::NotFound(id.to_string()))
    }

    pub async fn create(&self, create: CreateSchema) -> Result<schema::Model, ServiceError> {
        Self::validate(&create.schema, &create.fields_to_mask)?;
        let model = schema::Model {
            id: create.id.unwrap_or_else(Uuid::new_v4),
            name: create.name,
            created: Utc::now(),
            schema: create.schema,
            fields_to_mask: create.fields_to_mask,
            application_id: self.application_id,
        };
        let conn = self
            .db
            .conn()
            .map_err(|e| ServiceError::Internal(e.to_string()))?;
        self.repo.create(&conn, model).await.map_err(Into::into)
    }

    pub async fn update(
        &self,
        id: Uuid,
        params: UpdateSchema,
    ) -> Result<schema::Model, ServiceError> {
        Self::validate(&params.schema, &params.fields_to_mask)?;
        let conn = self
            .db
            .conn()
            .map_err(|e| ServiceError::Internal(e.to_string()))?;
        self.repo
            .update(&conn, id, self.application_id, params)
            .await
            .map_err(Into::into)
    }

    pub async fn delete(&self, id: Uuid) -> Result<(), ServiceError> {
        let conn = self
            .db
            .conn()
            .map_err(|e| ServiceError::Internal(e.to_string()))?;
        self.repo
            .delete(&conn, id, self.application_id)
            .await
            .map_err(Into::into)
    }

    // -------------------------------------------------------------------------
    // Validation
    // -------------------------------------------------------------------------

    fn validate(schema: &Value, fields_to_mask: &[String]) -> Result<(), ServiceError> {
        Self::validate_fields_to_mask(schema, fields_to_mask)?;

        jsonschema::meta::validate(schema).map_err(|_| {
            ServiceError::Validation("schema".to_string(), "Invalid JSON Schema".to_string())
        })?;

        Ok(())
    }

    fn validate_fields_to_mask(
        schema: &Value,
        fields_to_mask: &[String],
    ) -> Result<(), ServiceError> {
        let valid_fields: HashSet<String> = schema
            .get("properties")
            .and_then(|props| props.as_object())
            .map(|map| map.keys().cloned().collect())
            .unwrap_or_default();

        let invalid: Vec<&String> = fields_to_mask
            .iter()
            .filter(|f| !valid_fields.contains(*f))
            .collect();

        if !invalid.is_empty() {
            return Err(ServiceError::Validation(
                "fields_to_mask".to_string(),
                format!("Schema does not contain these fields: {invalid:?}"),
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn valid_schema() -> Value {
        json!({
            "type": "object",
            "properties": {
                "email": { "type": "string" },
                "password": { "type": "string" }
            }
        })
    }

    #[test]
    fn validate_ok_with_valid_fields() {
        let schema = valid_schema();
        let fields = vec!["email".to_string(), "password".to_string()];
        assert!(SchemasService::validate(&schema, &fields).is_ok());
    }

    #[test]
    fn validate_ok_with_empty_fields_to_mask() {
        let schema = valid_schema();
        assert!(SchemasService::validate(&schema, &[]).is_ok());
    }

    #[test]
    fn validate_fails_for_nonexistent_field() {
        let schema = valid_schema();
        let fields = vec!["email".to_string(), "nonexistent".to_string()];
        let err = SchemasService::validate(&schema, &fields).unwrap_err();
        assert!(err.to_string().contains("nonexistent"));
    }

    #[test]
    fn validate_fails_for_invalid_json_schema() {
        let invalid_schema = json!({ "$schema": "not-a-valid-schema-uri" });
        let err = SchemasService::validate(&invalid_schema, &[]).unwrap_err();
        assert!(err.to_string().contains("Invalid JSON Schema"));
    }

    #[test]
    fn validate_fields_to_mask_fails_when_schema_has_no_properties() {
        let schema = json!({ "type": "string" });
        let fields = vec!["anything".to_string()];
        assert!(SchemasService::validate_fields_to_mask(&schema, &fields).is_err());
    }

    #[test]
    fn validate_fields_to_mask_empty_always_passes() {
        let schema = json!({ "type": "string" });
        assert!(SchemasService::validate_fields_to_mask(&schema, &[]).is_ok());
    }
}
