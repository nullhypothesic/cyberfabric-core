use modkit_db::secure::{DBRunner, SecureDeleteExt, SecureEntityExt, SecureInsertExt, secure_update_with_scope};
use modkit_security::AccessScope;
use sea_orm::entity::prelude::Json;
use sea_orm::{ActiveValue, ColumnTrait, EntityTrait, QueryFilter};
use uuid::Uuid;

use crate::infra::db::entity::credential::{self, ActiveModel, Column, Entity};
use crate::infra::db::repo::error::RepositoryError;

/// Finds credentials by their definition ID and a list of tenant IDs.
/// Used for the fallback chain: own tenant + constructor tenant in one query.

pub struct UpdateCredentialDb {
    pub encrypted_value: Vec<u8>,
    pub masked_value: Json,
    pub propagate: bool,
}

pub struct CredentialsRepo;

impl CredentialsRepo {
    pub async fn find_credentials<C: DBRunner>(
        &self,
        conn: &C,
        definition_id: Uuid,
        tenant_ids: Vec<Uuid>,
    ) -> Result<Vec<credential::Model>, RepositoryError> {
        Entity::find()
            .filter(Column::DefinitionId.eq(definition_id))
            .filter(Column::TenantId.is_in(tenant_ids))
            .secure()
            .scope_with(&AccessScope::allow_all())
            .all(conn)
            .await
            .map_err(Into::into)
    }

    pub async fn find_by_definition_and_tenant<C: DBRunner>(
        &self,
        conn: &C,
        definition_id: Uuid,
        tenant_id: Uuid,
    ) -> Result<Option<credential::Model>, RepositoryError> {
        Entity::find()
            .filter(Column::DefinitionId.eq(definition_id))
            .secure()
            .scope_with(&AccessScope::for_tenant(tenant_id))
            .one(conn)
            .await
            .map_err(Into::into)
    }

    pub async fn find_by_definition_tenant_and_propagate<C: DBRunner>(
        &self,
        conn: &C,
        definition_id: Uuid,
        tenant_id: Uuid,
        propagate: bool,
    ) -> Result<Option<credential::Model>, RepositoryError> {
        Entity::find()
            .filter(Column::DefinitionId.eq(definition_id))
            .filter(Column::Propagate.eq(propagate))
            .secure()
            .scope_with(&AccessScope::for_tenant(tenant_id))
            .one(conn)
            .await
            .map_err(Into::into)
    }

    pub async fn create<C: DBRunner>(
        &self,
        conn: &C,
        model: credential::Model,
    ) -> Result<credential::Model, RepositoryError> {
        let id = model.id;
        let tenant_id = model.tenant_id;
        let scope = AccessScope::for_tenant(tenant_id);
        let active = ActiveModel {
            id: ActiveValue::Set(model.id),
            definition_id: ActiveValue::Set(model.definition_id),
            key_id: ActiveValue::Set(model.key_id),
            tenant_id: ActiveValue::Set(model.tenant_id),
            created: ActiveValue::Set(model.created),
            encrypted_value: ActiveValue::Set(model.encrypted_value),
            masked_value: ActiveValue::Set(model.masked_value),
            propagate: ActiveValue::Set(model.propagate),
        };

        Entity::insert(active)
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
            .await
            .map_err(Into::into)
    }

    pub async fn update<C: DBRunner>(
        &self,
        conn: &C,
        id: Uuid,
        tenant_id: Uuid,
        params: UpdateCredentialDb,
    ) -> Result<credential::Model, RepositoryError> {
        let scope = AccessScope::for_tenant(tenant_id);
        let active = ActiveModel {
            id: ActiveValue::Unchanged(id),
            tenant_id: ActiveValue::Unchanged(tenant_id),
            encrypted_value: ActiveValue::Set(params.encrypted_value),
            masked_value: ActiveValue::Set(params.masked_value),
            propagate: ActiveValue::Set(params.propagate),
            ..Default::default()
        };

        secure_update_with_scope::<Entity>(active, &scope, id, conn)
            .await
            .map_err(Into::into)
    }

    pub async fn delete<C: DBRunner>(
        &self,
        conn: &C,
        definition_id: Uuid,
        tenant_id: Uuid,
    ) -> Result<bool, RepositoryError> {
        let result = Entity::delete_many()
            .filter(Column::DefinitionId.eq(definition_id))
            .secure()
            .scope_with(&AccessScope::for_tenant(tenant_id))
            .exec(conn)
            .await
            .map_err(RepositoryError::from)?;
        Ok(result.rows_affected > 0)
    }
}
