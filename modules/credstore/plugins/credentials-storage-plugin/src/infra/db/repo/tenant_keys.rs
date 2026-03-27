use modkit_db::secure::{DBRunner, SecureEntityExt, SecureInsertExt};
use modkit_security::AccessScope;
use sea_orm::{ActiveValue, ColumnTrait, EntityTrait, QueryFilter};
use uuid::Uuid;

use crate::infra::db::entity::tenant_key::{self, ActiveModel, Column, Entity};
use crate::infra::db::repo::error::RepositoryError;

pub struct TenantKeysRepo;

impl TenantKeysRepo {
    pub async fn find_by_tenant_id<C: DBRunner>(
        &self,
        conn: &C,
        tenant_id: Uuid,
    ) -> Result<Option<tenant_key::Model>, RepositoryError> {
        Entity::find()
            .secure()
            .scope_with(&AccessScope::for_tenant(tenant_id))
            .one(conn)
            .await
            .map_err(Into::into)
    }

    pub async fn find_by_id<C: DBRunner>(
        &self,
        conn: &C,
        id: Uuid,
        tenant_id: Uuid,
    ) -> Result<Option<tenant_key::Model>, RepositoryError> {
        Entity::find()
            .filter(Column::Id.eq(id))
            .secure()
            .scope_with(&AccessScope::for_tenant(tenant_id))
            .one(conn)
            .await
            .map_err(Into::into)
    }

    pub async fn create<C: DBRunner>(
        &self,
        conn: &C,
        model: tenant_key::Model,
    ) -> Result<tenant_key::Model, RepositoryError> {
        let id = model.id;
        let tenant_id = model.tenant_id;
        let scope = AccessScope::for_tenant(tenant_id);
        let active = ActiveModel {
            id: ActiveValue::Set(model.id),
            tenant_id: ActiveValue::Set(model.tenant_id),
            created: ActiveValue::Set(model.created),
            key: ActiveValue::Set(model.key),
        };

        let inserted = Entity::insert(active)
            .secure()
            .scope_with_model(
                &scope,
                &ActiveModel {
                    id: ActiveValue::Set(id),
                    tenant_id: ActiveValue::Set(tenant_id),
                    ..Default::default()
                },
            )
            .map_err(RepositoryError::from)?
            .exec_with_returning(conn)
            .await?;

        Ok(inserted)
    }
}
