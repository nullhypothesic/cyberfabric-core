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
CREATE TABLE IF NOT EXISTS credential_definitions (
    id UUID PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    description TEXT NOT NULL,
    schema_id UUID NOT NULL,
    created TIMESTAMPTZ NOT NULL,
    default_value JSONB NOT NULL,
    CONSTRAINT fk_schema
        FOREIGN KEY (schema_id)
        REFERENCES schemas(id)
        ON DELETE RESTRICT
        ON UPDATE CASCADE
);

CREATE UNIQUE INDEX IF NOT EXISTS is_credential_definitions_name_unique
    ON credential_definitions (LOWER(name));
";

const DOWN: &str = r"
DROP INDEX IF EXISTS is_credential_definitions_name_unique;
DROP TABLE IF EXISTS credential_definitions;
";
