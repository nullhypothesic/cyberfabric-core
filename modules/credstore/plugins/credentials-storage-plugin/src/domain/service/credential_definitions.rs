use std::sync::Arc;

use modkit_db::{DBProvider, DbError};
use serde_json::Value;
use uuid::Uuid;

use chrono::Utc;

use crate::domain::credential_definition::CreateCredentialDefinition;
use crate::domain::error::ServiceError;
use crate::infra::db::entity::credential_definition;
use crate::infra::db::repo::credential_definitions::{
    CredentialDefinitionsRepo, UpdateCredentialDefinition,
};
use crate::infra::db::repo::schemas::SchemasRepo;

pub struct CredentialDefinitionsService {
    db: Arc<DBProvider<DbError>>,
    application_id: Uuid,
    repo: CredentialDefinitionsRepo,
    schemas_repo: SchemasRepo,
}

impl CredentialDefinitionsService {
    pub fn new(db: Arc<DBProvider<DbError>>, application_id: Uuid) -> Self {
        Self {
            db,
            application_id,
            repo: CredentialDefinitionsRepo,
            schemas_repo: SchemasRepo,
        }
    }

    pub async fn list(&self) -> Result<Vec<credential_definition::Model>, ServiceError> {
        let conn = self.db.conn().map_err(|e| ServiceError::Internal(e.to_string()))?;
        self.repo
            .find_all(&conn, self.application_id, None)
            .await
            .map_err(Into::into)
    }

    pub async fn get(&self, id: Uuid) -> Result<credential_definition::Model, ServiceError> {
        let conn = self.db.conn().map_err(|e| ServiceError::Internal(e.to_string()))?;
        let def = self
            .repo
            .find_by_id(&conn, id)
            .await?
            .ok_or_else(|| ServiceError::NotFound(id.to_string()))?;
        self.check_app_access(&def)?;
        Ok(def)
    }

    pub async fn create(
        &self,
        create: CreateCredentialDefinition,
    ) -> Result<credential_definition::Model, ServiceError> {
        self.validate_default_value(&create.schema_id, &create.default_value).await?;
        let model = credential_definition::Model {
            id: create.id.unwrap_or_else(Uuid::new_v4),
            name: create.name,
            description: create.description,
            schema_id: create.schema_id,
            created: Utc::now(),
            default_value: create.default_value,
            application_id: self.application_id,
            allowed_app_ids: create.allowed_app_ids,
        };
        let conn = self.db.conn().map_err(|e| ServiceError::Internal(e.to_string()))?;
        self.repo.create(&conn, model).await.map_err(Into::into)
    }

    pub async fn update(
        &self,
        id: Uuid,
        params: UpdateCredentialDefinition,
    ) -> Result<credential_definition::Model, ServiceError> {
        let def = self.get(id).await?;
        self.validate_default_value(&def.schema_id, &params.default_value).await?;
        let conn = self.db.conn().map_err(|e| ServiceError::Internal(e.to_string()))?;
        self.repo
            .update(&conn, id, self.application_id, params)
            .await
            .map_err(Into::into)
    }

    pub async fn delete(&self, id: Uuid) -> Result<(), ServiceError> {
        let conn = self.db.conn().map_err(|e| ServiceError::Internal(e.to_string()))?;
        self.repo
            .delete(&conn, id, self.application_id)
            .await
            .map_err(Into::into)
    }

    // -------------------------------------------------------------------------
    // Access check (also used by CredentialsService)
    // -------------------------------------------------------------------------

    pub fn check_app_access(&self, def: &credential_definition::Model) -> Result<(), ServiceError> {
        if def.application_id == self.application_id
            || def.allowed_app_ids.contains(&self.application_id)
        {
            Ok(())
        } else {
            Err(ServiceError::Forbidden(
                "application is not allowed to access this definition".to_string(),
            ))
        }
    }

    // -------------------------------------------------------------------------
    // Validation
    // -------------------------------------------------------------------------

    async fn validate_default_value(
        &self,
        schema_id: &Uuid,
        value: &Value,
    ) -> Result<(), ServiceError> {
        let conn = self.db.conn().map_err(|e| ServiceError::Internal(e.to_string()))?;

        let schema = self
            .schemas_repo
            .find_by_id(&conn, *schema_id)
            .await?
            .ok_or_else(|| {
                ServiceError::Validation(
                    "schema_id".to_string(),
                    "Schema does not exist".to_string(),
                )
            })?;

        if jsonschema::is_valid(&schema.schema, value) {
            Ok(())
        } else {
            Err(ServiceError::Validation(
                "default_value".to_string(),
                "default_value does not conform to schema".to_string(),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn make_def(app_id: Uuid, allowed: Vec<Uuid>) -> credential_definition::Model {
        credential_definition::Model {
            id: Uuid::new_v4(),
            name: "test".to_string(),
            description: "desc".to_string(),
            schema_id: Uuid::new_v4(),
            created: chrono::Utc::now(),
            default_value: json!({}),
            application_id: app_id,
            allowed_app_ids: allowed,
        }
    }

    /// Mirrors `check_app_access` pure logic without requiring a service instance.
    fn can_access(def: &credential_definition::Model, caller: Uuid) -> bool {
        def.application_id == caller || def.allowed_app_ids.contains(&caller)
    }

    #[test]
    fn owner_app_has_access() {
        let app_id = Uuid::new_v4();
        assert!(can_access(&make_def(app_id, vec![]), app_id));
    }

    #[test]
    fn allowed_app_has_access() {
        let owner = Uuid::new_v4();
        let caller = Uuid::new_v4();
        assert!(can_access(&make_def(owner, vec![caller]), caller));
    }

    #[test]
    fn unrelated_app_is_denied() {
        let owner = Uuid::new_v4();
        let caller = Uuid::new_v4();
        assert!(!can_access(&make_def(owner, vec![]), caller));
    }
}
