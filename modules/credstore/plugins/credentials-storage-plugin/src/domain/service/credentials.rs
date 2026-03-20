use std::sync::Arc;

use chrono::Utc;
use modkit_db::{DBProvider, DbError};
use rand::{Rng, distr::Alphanumeric};
use serde_json::Value;
use uuid::{Uuid, uuid};

use crate::domain::credential::{CreateCredential, Credential, UpdateCredential};
use crate::domain::error::ServiceError;
use crate::infra::crypto;
use crate::infra::db::entity::{credential, credential_definition, tenant_key};
use crate::infra::db::repo::{
    credential_definitions::CredentialDefinitionsRepo,
    credentials::{CredentialsRepo, UpdateCredentialDb},
    schemas::SchemasRepo,
    tenant_keys::TenantKeysRepo,
};

/// Parent tenant ID for credential inheritance hierarchy.
/// Credentials from this tenant can be inherited by child tenants when propagate=true.
const CONSTRUCTOR_TENANT_ID: Uuid = uuid!("46f09f8d-e59b-4c3d-a7a4-c5e9fafbd2c4");

/// Mask string used to hide sensitive values
const MASK_STRING: &str = "***";

pub struct CredentialsService {
    db: Arc<DBProvider<DbError>>,
    application_id: Uuid,
    credentials_repo: CredentialsRepo,
    tenant_keys_repo: TenantKeysRepo,
    definitions_repo: CredentialDefinitionsRepo,
    schemas_repo: SchemasRepo,
}

impl CredentialsService {
    pub fn new(db: Arc<DBProvider<DbError>>, application_id: Uuid) -> Self {
        Self {
            db,
            application_id,
            credentials_repo: CredentialsRepo,
            tenant_keys_repo: TenantKeysRepo,
            definitions_repo: CredentialDefinitionsRepo,
            schemas_repo: SchemasRepo,
        }
    }

    pub async fn create_credential(
        &self,
        tenant_id: Uuid,
        create_credential: CreateCredential,
    ) -> Result<Credential, ServiceError> {
        let (masked_value, encrypted_value, definition, tenant_key) = self
            .prepare_value(
                &create_credential.definition_name,
                tenant_id,
                &create_credential.value,
            )
            .await?;

        let conn = self.db.conn().map_err(|e| ServiceError::Internal(e.to_string()))?;

        let model = credential::Model {
            id: create_credential.id.unwrap_or_else(Uuid::new_v4),
            definition_id: definition.id,
            key_id: tenant_key.id,
            tenant_id,
            created: Utc::now(),
            encrypted_value,
            masked_value,
            propagate: create_credential.propagate,
        };

        let created = self
            .credentials_repo
            .create(&conn, model)
            .await
            .map_err(ServiceError::from)?;

        Ok(Credential::from(created).add_definition_name(definition.name))
    }

    pub async fn update_credential(
        &self,
        update_credential: UpdateCredential,
    ) -> Result<Credential, ServiceError> {
        let (masked_value, encrypted_value, definition, _tenant_key) = self
            .prepare_value(
                &update_credential.definition_name,
                update_credential.tenant_id,
                &update_credential.value,
            )
            .await?;

        let conn = self.db.conn().map_err(|e| ServiceError::Internal(e.to_string()))?;

        let existing = self
            .credentials_repo
            .find_by_definition_and_tenant(&conn, definition.id, update_credential.tenant_id)
            .await?
            .ok_or_else(|| ServiceError::NotFound(update_credential.definition_name.clone()))?;

        let updated = self
            .credentials_repo
            .update(
                &conn,
                existing.id,
                update_credential.tenant_id,
                UpdateCredentialDb {
                    encrypted_value,
                    masked_value,
                    propagate: update_credential.propagate,
                },
            )
            .await
            .map_err(ServiceError::from)?;

        Ok(Credential::from(updated).add_definition_name(definition.name))
    }

    pub async fn delete_credential(
        &self,
        definition_name: &str,
        tenant_id: Uuid,
    ) -> Result<(), ServiceError> {
        let definition = self.get_definition(definition_name).await?;

        let conn = self.db.conn().map_err(|e| ServiceError::Internal(e.to_string()))?;

        let deleted = self
            .credentials_repo
            .delete(&conn, definition.id, tenant_id)
            .await
            .map_err(ServiceError::from)?;

        if !deleted {
            return Err(ServiceError::NotFound(definition_name.to_string()));
        }

        Ok(())
    }

    pub async fn get_credential_for_app(
        &self,
        definition_name: &str,
        tenant_id: Uuid,
    ) -> Result<Value, ServiceError> {
        let definition = self.get_definition(definition_name).await?;
        // We check if definition.app_id = our app_id, if not will return 403
        let app_id_same = definition.application_id == self.application_id;
        let app_id_in_allowed = definition.allowed_app_ids.contains(&self.application_id);

        if !app_id_same && !app_id_in_allowed {
            return Err(ServiceError::Forbidden(
                "You don't have rights to get that credential".to_string(),
            ));
        }

        let credential = self
            .get_credential_by_definition(tenant_id, &definition)
            .await?;

        // Decrypt the credential value if it has an encryption key
        let credential = self.decrypt_single_credential(credential).await?;

        Ok(credential)
    }

    // -------------------------------------------------------------------------
    // Helpers
    // -------------------------------------------------------------------------

    async fn get_credential_by_definition(
        &self,
        tenant_id: Uuid,
        definition: &credential_definition::Model,
    ) -> Result<SelectedCredential, ServiceError> {
        let conn = self.db.conn().map_err(|e| ServiceError::Internal(e.to_string()))?;

        let credentials = self
            .credentials_repo
            .find_credentials(&conn, definition.id, vec![tenant_id, CONSTRUCTOR_TENANT_ID])
            .await
            .map_err(ServiceError::from)?;

        Ok(self.select_appropriate_credential(&credentials, tenant_id, definition))
    }

    /// Selects the most appropriate credential for a tenant from a list of credentials and a definition.
    fn select_appropriate_credential(
        &self,
        credentials: &[credential::Model],
        tenant_id: Uuid,
        definition: &credential_definition::Model,
    ) -> SelectedCredential {
        // 1: Looking for credential for our tenant
        if let Some(credential) = credentials
            .iter()
            .filter(|x| x.definition_id == definition.id)
            .find(|x| x.tenant_id == tenant_id)
            .cloned()
        {
            return SelectedCredential::OwnTenant(credential);
        }

        // 2: If not found, looking for parent tenant, now hardcoded to Constructor tenant
        if let Some(credential) = credentials
            .iter()
            .filter(|x| x.definition_id == definition.id)
            .find(|x| x.tenant_id == CONSTRUCTOR_TENANT_ID)
            .cloned()
            && credential.propagate
        {
            return SelectedCredential::Inherited(credential);
        }

        // 3: If not found, or propagate false, convert definition to credential view
        SelectedCredential::Default(definition.default_value.clone())
    }

    async fn decrypt_single_credential(
        &self,
        credential: SelectedCredential,
    ) -> Result<Value, ServiceError> {
        match credential {
            SelectedCredential::OwnTenant(c) => {
                let conn = self.db.conn().map_err(|e| ServiceError::Internal(e.to_string()))?;
                let tenant_key = self
                    .tenant_keys_repo
                    .find_by_id(&conn, c.key_id, c.tenant_id)
                    .await?
                    .ok_or_else(|| ServiceError::Internal(format!(
                        "Tenant key with id {} not found for credential decryption",
                        c.key_id
                    )))?;
                crypto::decrypt_value(&c.encrypted_value, &tenant_key.key).map_err(Into::into)
            }
            SelectedCredential::Inherited(c) => {
                let conn = self.db.conn().map_err(|e| ServiceError::Internal(e.to_string()))?;
                let tenant_key = self
                    .tenant_keys_repo
                    .find_by_id(&conn, c.key_id, CONSTRUCTOR_TENANT_ID)
                    .await?
                    .ok_or_else(|| ServiceError::Internal(format!(
                        "Tenant key with id {} not found for credential decryption",
                        c.key_id
                    )))?;
                crypto::decrypt_value(&c.encrypted_value, &tenant_key.key).map_err(Into::into)
            }
            // Credential from definition — already has decrypted value
            SelectedCredential::Default(value) => Ok(value),
        }
    }

    async fn get_definition(
        &self,
        name: &str,
    ) -> Result<credential_definition::Model, ServiceError> {
        let conn = self.db.conn().map_err(|e| ServiceError::Internal(e.to_string()))?;
        self.definitions_repo
            .find_by_name(&conn, name)
            .await?
            .ok_or_else(|| ServiceError::NotFound(name.to_string()))
    }

    async fn prepare_value(
        &self,
        definition_name: &str,
        tenant_id: Uuid,
        value: &Value,
    ) -> Result<(Value, Vec<u8>, credential_definition::Model, tenant_key::Model), ServiceError>
    {
        let definition = self.get_definition(definition_name).await?;

        let conn = self.db.conn().map_err(|e| ServiceError::Internal(e.to_string()))?;
        let schema = self
            .schemas_repo
            .find_by_id(&conn, definition.schema_id)
            .await?
            .ok_or_else(|| ServiceError::Internal("schema not found".to_string()))?;

        if !jsonschema::is_valid(&schema.schema, value) {
            return Err(ServiceError::Validation(
                "value".to_string(),
                "value isn't corresponde to schema".to_string(),
            ));
        }

        let masked_value = self.mask_value(value.clone(), schema.fields_to_mask);

        let tenant_key = self.get_or_create_tenant_key(tenant_id).await?;

        let encrypted_value = crypto::encrypt_value(value, &tenant_key.key)?;

        Ok((masked_value, encrypted_value, definition, tenant_key))
    }

    async fn get_or_create_tenant_key(
        &self,
        tenant_id: Uuid,
    ) -> Result<tenant_key::Model, ServiceError> {
        let conn = self.db.conn().map_err(|e| ServiceError::Internal(e.to_string()))?;

        if let Some(tenant_key) = self
            .tenant_keys_repo
            .find_by_tenant_id(&conn, tenant_id)
            .await?
        {
            return Ok(tenant_key);
        }

        let key: String = rand::rng()
            .sample_iter(&Alphanumeric)
            .take(32)
            .map(char::from)
            .collect();

        let model = tenant_key::Model {
            id: Uuid::new_v4(),
            tenant_id,
            created: Utc::now(),
            key,
        };

        self.tenant_keys_repo
            .create(&conn, model)
            .await
            .map_err(Into::into)
    }

    fn mask_value(&self, value: Value, fields_to_mask: Vec<String>) -> Value {
        Self::mask_value_internal(value, &fields_to_mask)
    }

    fn mask_value_internal(value: Value, fields_to_mask: &[String]) -> Value {
        if fields_to_mask.is_empty() {
            Self::mask_all_values(value)
        } else {
            Self::mask_specific_fields(value, fields_to_mask)
        }
    }

    fn mask_all_values(value: Value) -> Value {
        match value {
            Value::Object(map) => {
                let masked = map
                    .into_iter()
                    .map(|(k, _)| (k, Value::String(MASK_STRING.to_string())))
                    .collect();
                Value::Object(masked)
            }
            Value::Array(arr) => {
                let masked = arr
                    .into_iter()
                    .map(|_| Value::String(MASK_STRING.to_string()))
                    .collect();
                Value::Array(masked)
            }
            _ => Value::String(MASK_STRING.to_string()),
        }
    }

    fn mask_specific_fields(value: Value, fields_to_mask: &[String]) -> Value {
        match value {
            Value::Object(map) => {
                let masked = map
                    .into_iter()
                    .map(|(k, v)| {
                        if fields_to_mask.contains(&k) {
                            (k, Value::String(MASK_STRING.to_string()))
                        } else {
                            (k, v)
                        }
                    })
                    .collect();
                Value::Object(masked)
            }
            _ => value,
        }
    }
}

/// Represents the credential selected from the fallback chain.
enum SelectedCredential {
    OwnTenant(credential::Model),
    Inherited(credential::Model),
    Default(Value),
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn mask_value_replaces_listed_fields() {
        let value = json!({ "email": "user@example.com", "password": "secret", "name": "Alice" });
        let fields = vec!["password".to_string()];
        let masked = CredentialsService::mask_value_internal(value, &fields);
        assert_eq!(masked["password"], "***");
        assert_eq!(masked["email"], "user@example.com");
        assert_eq!(masked["name"], "Alice");
    }

    #[test]
    fn mask_value_empty_fields_masks_all() {
        let value = json!({ "email": "user@example.com", "password": "secret" });
        let masked = CredentialsService::mask_value_internal(value, &[]);
        assert_eq!(masked["email"], "***");
        assert_eq!(masked["password"], "***");
    }

    #[test]
    fn mask_value_ignores_missing_fields() {
        let value = json!({ "email": "user@example.com" });
        let fields = vec!["password".to_string()];
        let masked = CredentialsService::mask_value_internal(value.clone(), &fields);
        assert_eq!(masked, value);
    }

    #[test]
    fn mask_all_values_for_array() {
        let value = json!(["a", "b", "c"]);
        let masked = CredentialsService::mask_all_values(value);
        assert_eq!(masked, json!(["***", "***", "***"]));
    }

    #[test]
    fn mask_all_values_for_primitive() {
        let value = json!("plain");
        let masked = CredentialsService::mask_all_values(value);
        assert_eq!(masked, json!("***"));
    }
}
