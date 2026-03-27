use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::ConnectionTrait;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(UP)
            .await
            .map(|_| ())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(DOWN)
            .await
            .map(|_| ())
    }
}

const UP: &str = r"
CREATE TABLE IF NOT EXISTS credentials (
    id UUID PRIMARY KEY,
    definition_id UUID NOT NULL,
    key_id UUID NOT NULL,
    tenant_id UUID NOT NULL,
    created TIMESTAMPTZ NOT NULL,
    encrypted_value BYTEA NOT NULL,
    masked_value JSONB NOT NULL,
    propagate BOOLEAN NOT NULL DEFAULT FALSE,
    CONSTRAINT fk_definition
        FOREIGN KEY (definition_id)
        REFERENCES credential_definitions(id)
        ON DELETE RESTRICT
        ON UPDATE CASCADE,
    CONSTRAINT fk_tenant_key
        FOREIGN KEY (key_id)
        REFERENCES tenant_keys(id)
        ON DELETE RESTRICT
        ON UPDATE CASCADE
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_credentials_definition_tenant
    ON credentials (definition_id, tenant_id);
";

const DOWN: &str = r"
DROP INDEX IF EXISTS idx_credentials_definition_tenant;
DROP TABLE IF EXISTS credentials;
";
