use chrono::{DateTime, Utc};
use modkit_db_macros::Scopable;
use sea_orm::entity::prelude::*;
use uuid::Uuid;

/// Sea-ORM entity for the `tenant_keys` table.
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Scopable)]
#[sea_orm(table_name = "tenant_keys")]
#[secure(tenant_col = "tenant_id", resource_col = "id", no_owner, no_type)]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub created: DateTime<Utc>,
    pub key: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
