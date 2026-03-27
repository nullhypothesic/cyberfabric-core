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
ALTER TABLE credential_definitions ADD COLUMN IF NOT EXISTS application_id UUID;
UPDATE credential_definitions SET application_id = '00000000-0000-0000-0000-000000000000' WHERE application_id IS NULL;
ALTER TABLE credential_definitions ALTER COLUMN application_id SET NOT NULL;
";

const DOWN: &str = r"
ALTER TABLE credential_definitions DROP COLUMN IF EXISTS application_id;
";
