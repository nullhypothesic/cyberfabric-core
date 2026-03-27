use modkit_db::secure::{
    DBRunner, SecureDeleteExt, SecureEntityExt, SecureInsertExt, secure_update_with_scope,
};
use modkit_security::AccessScope;
use sea_orm::entity::prelude::Json;
use sea_orm::sea_query::{Expr, Func};
use sea_orm::{ActiveValue, ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
use tracing::error;
use uuid::Uuid;

use crate::infra::db::entity::credential_definition::{self, ActiveModel, Column, Entity};
use crate::infra::db::repo::error::RepositoryError;

pub struct UpdateCredentialDefinition {
    pub name: String,
    pub description: String,
    pub default_value: Json,
    pub allowed_app_ids: Vec<Uuid>,
}

pub struct CredentialDefinitionsRepo;

impl CredentialDefinitionsRepo {
    pub async fn find_by_id<C: DBRunner>(
        &self,
        conn: &C,
        id: Uuid,
    ) -> Result<Option<credential_definition::Model>, RepositoryError> {
        Entity::find()
            .filter(Column::Id.eq(id))
            .secure()
            .scope_with(&AccessScope::allow_all())
            .one(conn)
            .await
            .map_err(Into::into)
    }

    /// Case-insensitive name lookup.
    pub async fn find_by_name<C: DBRunner>(
        &self,
        conn: &C,
        name: &str,
    ) -> Result<Option<credential_definition::Model>, RepositoryError> {
        Entity::find()
            .filter(Expr::expr(Func::lower(Expr::col(Column::Name))).eq(name.to_lowercase()))
            .secure()
            .scope_with(&AccessScope::allow_all())
            .one(conn)
            .await
            .map_err(Into::into)
    }

    /// Returns all definitions visible to `application_id`:
    /// rows where `application_id = app_id OR app_id = ANY(allowed_app_ids)`.
    /// Optionally pre-filtered by name.
    pub async fn find_all<C: DBRunner>(
        &self,
        conn: &C,
        application_id: Uuid,
        definition_names: Option<Vec<String>>,
    ) -> Result<Vec<credential_definition::Model>, RepositoryError> {
        use sea_orm::Condition;
        use sea_orm::sea_query::Expr;

        let mut q = Entity::find()
            .filter(
                Condition::any()
                    .add(Column::ApplicationId.eq(application_id))
                    .add(Expr::cust_with_values(
                        "? = ANY(allowed_app_ids)",
                        [application_id],
                    )),
            )
            .order_by_asc(Column::Name);

        if let Some(names) = definition_names.filter(|v| !v.is_empty()) {
            q = q.filter(Column::Name.is_in(names));
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
        model: credential_definition::Model,
    ) -> Result<credential_definition::Model, RepositoryError> {
        let active = ActiveModel {
            id: ActiveValue::Set(model.id),
            name: ActiveValue::Set(model.name),
            description: ActiveValue::Set(model.description),
            schema_id: ActiveValue::Set(model.schema_id),
            created: ActiveValue::Set(model.created),
            default_value: ActiveValue::Set(model.default_value),
            application_id: ActiveValue::Set(model.application_id),
            allowed_app_ids: ActiveValue::Set(model.allowed_app_ids),
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
        params: UpdateCredentialDefinition,
    ) -> Result<credential_definition::Model, RepositoryError> {
        self.validate_access(conn, id, application_id).await?;

        let active = ActiveModel {
            id: ActiveValue::Unchanged(id),
            name: ActiveValue::Set(params.name),
            description: ActiveValue::Set(params.description),
            default_value: ActiveValue::Set(params.default_value),
            allowed_app_ids: ActiveValue::Set(params.allowed_app_ids),
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
        let Some(existing) = self.find_by_id(conn, id).await? else {
            return Err(RepositoryError::NotFound);
        };
        if existing.application_id != application_id {
            error!(
                "Access denied for definition {} — expected app {}, got {}",
                id, existing.application_id, application_id
            );
            return Err(RepositoryError::Forbidden);
        }
        Ok(())
    }
}
