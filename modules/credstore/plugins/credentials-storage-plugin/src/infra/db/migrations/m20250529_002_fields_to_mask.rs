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
ALTER TABLE schemas ADD COLUMN IF NOT EXISTS fields_to_mask VARCHAR[] DEFAULT '{}';
UPDATE schemas SET fields_to_mask = '{}' WHERE fields_to_mask IS NULL;
ALTER TABLE schemas ALTER COLUMN fields_to_mask SET NOT NULL;
";

const DOWN: &str = r"
ALTER TABLE schemas DROP COLUMN IF EXISTS fields_to_mask;
";
