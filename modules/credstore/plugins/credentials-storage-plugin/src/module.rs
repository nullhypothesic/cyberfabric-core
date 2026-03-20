use std::sync::{Arc, OnceLock};

use async_trait::async_trait;
use axum::Router;
use credstore_sdk::{CredStorePluginClientV1, CredStorePluginSpecV1};
use modkit::Module;
use modkit::api::OpenApiRegistry;
use modkit::client_hub::ClientScope;
use modkit::context::ModuleCtx;
use modkit::gts::BaseModkitPluginV1;
use modkit_db::{DBProvider, DbError};
use tracing::info;
use types_registry_sdk::{RegisterResult, TypesRegistryClient};
use uuid::Uuid;

use crate::api::rest::routes;
use crate::config::CredentialsStoragePluginConfig;
use crate::domain::service::CredentialsStorageService;

/// Credentials Storage credstore plugin.
///
/// Provides DB-backed credential management (schemas → definitions → encrypted credentials)
/// and implements [`CredStorePluginClientV1`] for the credstore gateway.
#[modkit::module(
    name = "credentials-storage-plugin",
    deps = ["types-registry"],
    capabilities = [db, rest]
)]
pub struct CredentialsStoragePlugin {
    service: OnceLock<Arc<CredentialsStorageService>>,
    application_id: OnceLock<Uuid>,
}

impl Default for CredentialsStoragePlugin {
    fn default() -> Self {
        Self {
            service: OnceLock::new(),
            application_id: OnceLock::new(),
        }
    }
}

impl modkit::contracts::DatabaseCapability for CredentialsStoragePlugin {
    fn migrations(&self) -> Vec<Box<dyn sea_orm_migration::MigrationTrait>> {
        use sea_orm_migration::MigratorTrait;
        info!("Providing credentials-storage-plugin database migrations");
        crate::infra::db::migrations::Migrator::migrations()
    }
}

#[async_trait]
impl Module for CredentialsStoragePlugin {
    async fn init(&self, ctx: &ModuleCtx) -> anyhow::Result<()> {
        let cfg: CredentialsStoragePluginConfig = ctx.config()?;

        info!(
            vendor = %cfg.vendor,
            priority = cfg.priority,
            application_id = %cfg.application_id,
            "Loading credentials-storage-plugin configuration"
        );

        let db: Arc<DBProvider<DbError>> = Arc::new(ctx.db_required()?);

        let service = Arc::new(CredentialsStorageService::new(db, cfg.application_id));

        // Generate GTS instance ID and register with types-registry
        let instance_id =
            CredStorePluginSpecV1::gts_make_instance_id("x.core._.credentials_storage.v1");

        let registry = ctx.client_hub().get::<dyn TypesRegistryClient>()?;
        let instance = BaseModkitPluginV1::<CredStorePluginSpecV1> {
            id: instance_id.clone(),
            vendor: cfg.vendor.clone(),
            priority: cfg.priority,
            properties: CredStorePluginSpecV1,
        };
        let instance_json = serde_json::to_value(&instance)?;

        let results = registry.register(vec![instance_json]).await?;
        RegisterResult::ensure_all_ok(&results)?;

        // Commit state
        self.application_id
            .set(cfg.application_id)
            .map_err(|_| anyhow::anyhow!("{} module already initialized", Self::MODULE_NAME))?;
        self.service
            .set(service.clone())
            .map_err(|_| anyhow::anyhow!("{} module already initialized", Self::MODULE_NAME))?;

        // Register as CredStorePluginClientV1 in ClientHub
        let api: Arc<dyn CredStorePluginClientV1> = service;
        ctx.client_hub()
            .register_scoped::<dyn CredStorePluginClientV1>(ClientScope::gts_id(&instance_id), api);

        info!(instance_id = %instance_id, "credentials-storage-plugin registered");
        Ok(())
    }
}

#[async_trait]
impl modkit::contracts::RestApiCapability for CredentialsStoragePlugin {
    fn register_rest(
        &self,
        _ctx: &ModuleCtx,
        router: Router,
        openapi: &dyn OpenApiRegistry,
    ) -> anyhow::Result<Router> {
        let service = self
            .service
            .get()
            .ok_or_else(|| anyhow::anyhow!("{} not initialized", Self::MODULE_NAME))?
            .clone();

        let app_id = *self
            .application_id
            .get()
            .ok_or_else(|| anyhow::anyhow!("{} not initialized (app_id)", Self::MODULE_NAME))?;

        let router = routes::register_routes(router, openapi, service, app_id);
        info!("credentials-storage-plugin REST routes registered");
        Ok(router)
    }
}
