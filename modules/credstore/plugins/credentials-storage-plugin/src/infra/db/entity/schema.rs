use chrono::{DateTime, Utc};
use modkit_db_macros::Scopable;
use sea_orm::entity::prelude::*;
use uuid::Uuid;

/// Sea-ORM entity for the `schemas` table.
///
/// Schemas are application-scoped (not tenant-scoped) — use `AccessScope::allow_all()`
/// and filter explicitly by `application_id` in repositories.
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Scopable)]
#[sea_orm(table_name = "schemas")]
#[secure(no_tenant, resource_col = "id", no_owner, no_type)]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub name: String,
    pub created: DateTime<Utc>,
    pub schema: Json,
    pub fields_to_mask: Vec<String>,
    pub application_id: Uuid,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
