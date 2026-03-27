use modkit_db::secure::{
    DBRunner, SecureDeleteExt, SecureEntityExt, SecureInsertExt, secure_update_with_scope,
};
use modkit_security::AccessScope;
use sea_orm::entity::prelude::Json;
use sea_orm::{ActiveValue, ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
use tracing::error;
use uuid::Uuid;

use crate::infra::db::entity::schema::{self, ActiveModel, Column, Entity};
use crate::infra::db::repo::error::RepositoryError;

pub struct UpdateSchema {
    pub name: String,
    pub schema: Json,
    pub fields_to_mask: Vec<String>,
}

pub struct SchemasRepo;

impl SchemasRepo {
    pub async fn find_by_id<C: DBRunner>(
        &self,
        conn: &C,
        id: Uuid,
    ) -> Result<Option<schema::Model>, RepositoryError> {
        Entity::find()
            .filter(Column::Id.eq(id))
            .secure()
            .scope_with(&AccessScope::allow_all())
            .one(conn)
            .await
            .map_err(Into::into)
    }

    pub async fn find_all<C: DBRunner>(
        &self,
        conn: &C,
        ids: Option<Vec<Uuid>>,
    ) -> Result<Vec<schema::Model>, RepositoryError> {
        let mut q = Entity::find().order_by_asc(Column::Name);
        if let Some(ids) = ids.filter(|v| !v.is_empty()) {
            q = q.filter(Column::Id.is_in(ids));
        }
        q.secure()
            .scope_with(&AccessScope::allow_all())
            .all(conn)
            .await
            .map_err(Into::into)
    }

    pub async fn create<C: DBRunner>(
        &self,
        conn: &C,
        model: schema::Model,
    ) -> Result<schema::Model, RepositoryError> {
        let active = ActiveModel {
            id: ActiveValue::Set(model.id),
            name: ActiveValue::Set(model.name),
            created: ActiveValue::Set(model.created),
            schema: ActiveValue::Set(model.schema),
            fields_to_mask: ActiveValue::Set(model.fields_to_mask),
            application_id: ActiveValue::Set(model.application_id),
        };

        Entity::insert(active)
            .secure()
            .scope_unchecked(&AccessScope::allow_all())
            .map_err(RepositoryError::from)?
            .exec_with_returning(conn)
            .await
            .map_err(Into::into)
    }

    pub async fn update<C: DBRunner>(
        &self,
        conn: &C,
        id: Uuid,
        application_id: Uuid,
        params: UpdateSchema,
    ) -> Result<schema::Model, RepositoryError> {
        self.validate_access(conn, id, application_id).await?;

        let active = ActiveModel {
            id: ActiveValue::Unchanged(id),
            name: ActiveValue::Set(params.name),
            schema: ActiveValue::Set(params.schema),
            fields_to_mask: ActiveValue::Set(params.fields_to_mask),
            ..Default::default()
        };

        secure_update_with_scope::<Entity>(active, &AccessScope::allow_all(), id, conn)
            .await
            .map_err(Into::into)
    }

    pub async fn delete<C: DBRunner>(
        &self,
        conn: &C,
        id: Uuid,
        application_id: Uuid,
    ) -> Result<(), RepositoryError> {
        self.validate_access(conn, id, application_id).await?;

        let result = Entity::delete_many()
            .filter(Column::Id.eq(id))
            .secure()
            .scope_with(&AccessScope::allow_all())
            .exec(conn)
            .await
            .map_err(|e| RepositoryError::Scope(e.to_string()))?;

        if result.rows_affected == 0 {
            return Err(RepositoryError::NotFound);
        }

        Ok(())
    }

    async fn validate_access<C: DBRunner>(
        &self,
        conn: &C,
        id: Uuid,
        application_id: Uuid,
    ) -> Result<(), RepositoryError> {
        let Some(schema) = self.find_by_id(conn, id).await? else {
            return Err(RepositoryError::NotFound);
        };
        if schema.application_id != application_id {
            error!(
                "Access denied for schema {} — expected app {}, got {}",
                id, schema.application_id, application_id
            );
            return Err(RepositoryError::Forbidden);
        }
        Ok(())
    }
}
