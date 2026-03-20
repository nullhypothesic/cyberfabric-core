pub mod credential_definitions;
pub mod credentials;
pub mod schemas;

use std::sync::Arc;

use async_trait::async_trait;
use credstore_sdk::{
    CredStoreError, CredStorePluginClientV1, SecretMetadata, SecretRef, SecretValue, SharingMode,
};
use modkit_db::{DBProvider, DbError};
use modkit_security::SecurityContext;
use uuid::Uuid;

use self::credential_definitions::CredentialDefinitionsService;
use self::credentials::CredentialsService;
use self::schemas::SchemasService;

pub struct CredentialsStorageService {
    pub schemas: SchemasService,
    pub definitions: CredentialDefinitionsService,
    pub credentials: CredentialsService,
}

impl CredentialsStorageService {
    pub fn new(db: Arc<DBProvider<DbError>>, application_id: Uuid) -> Self {
        Self {
            schemas: SchemasService::new(Arc::clone(&db), application_id),
            definitions: CredentialDefinitionsService::new(Arc::clone(&db), application_id),
            credentials: CredentialsService::new(db, application_id),
        }
    }
}

#[async_trait]
impl CredStorePluginClientV1 for CredentialsStorageService {
    async fn get(
        &self,
        ctx: &SecurityContext,
        key: &SecretRef,
    ) -> Result<Option<SecretMetadata>, CredStoreError> {
        let tenant_id = ctx.subject_tenant_id();
        let definition_name = key.as_ref();

        let value = match self
            .credentials
            .get_credential_for_app(definition_name, tenant_id)
            .await
        {
            Ok(v) => v,
            Err(e) => return Err(e.into()),
        };

        let secret_value = SecretValue::from(serde_json::to_string(&value).unwrap_or_default());

        Ok(Some(SecretMetadata {
            value: secret_value,
            owner_id: credstore_sdk::OwnerId(ctx.subject_id()),
            sharing: SharingMode::Tenant,
            owner_tenant_id: credstore_sdk::TenantId(tenant_id),
        }))
    }
}
