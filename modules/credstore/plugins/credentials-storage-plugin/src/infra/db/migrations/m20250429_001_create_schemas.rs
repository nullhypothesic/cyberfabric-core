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
CREATE TABLE IF NOT EXISTS schemas (
    id UUID PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    created TIMESTAMPTZ NOT NULL,
    schema JSONB NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS ix_schemas_name_unique ON schemas (LOWER(name));
";

const DOWN: &str = r"
DROP INDEX IF EXISTS ix_schemas_name_unique;
DROP TABLE IF EXISTS schemas;
";
