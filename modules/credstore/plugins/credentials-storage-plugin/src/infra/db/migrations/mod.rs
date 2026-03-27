use sea_orm_migration::prelude::*;

mod m20250429_001_create_schemas;
mod m20250529_002_fields_to_mask;
mod m20250604_003_credential_definitions;
mod m20250611_004_tenant_keys;
mod m20250611_005_credentials;
mod m20250722_006_app_id_schemas;
mod m20250722_007_app_id_definitions;
mod m20250722_008_allowed_app_ids;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20250429_001_create_schemas::Migration),
            Box::new(m20250529_002_fields_to_mask::Migration),
            Box::new(m20250604_003_credential_definitions::Migration),
            Box::new(m20250611_004_tenant_keys::Migration),
            Box::new(m20250611_005_credentials::Migration),
            Box::new(m20250722_006_app_id_schemas::Migration),
            Box::new(m20250722_007_app_id_definitions::Migration),
            Box::new(m20250722_008_allowed_app_ids::Migration),
        ]
    }
}
