use std::sync::Arc;

use axum::Json;
use axum::extract::{Extension, Path};
use axum::response::IntoResponse;
use modkit::api::prelude::*;
use modkit_security::SecurityContext;
use uuid::Uuid;

use crate::domain::credential::{CreateCredential, UpdateCredential};
use crate::domain::credential_definition::CreateCredentialDefinition;
use crate::domain::schema::CreateSchema;
use crate::domain::service::CredentialsStorageService;

use super::dto::{
    CreateCredentialDefinitionRequest, CreateCredentialRequest, CreateSchemaRequest,
    CredentialDefinitionDto, CredentialDto, SchemaDto, UpdateCredentialDefinitionRequest,
    UpdateCredentialRequest, UpdateSchemaRequest,
};

// ---------------------------------------------------------------------------
// Schemas
// ---------------------------------------------------------------------------

pub async fn list_schemas(
    Extension(_ctx): Extension<SecurityContext>,
    Extension(svc): Extension<Arc<CredentialsStorageService>>,
) -> ApiResult<impl IntoResponse> {
    let schemas = svc.schemas.list().await?;
    let dtos: Vec<SchemaDto> = schemas.into_iter().map(SchemaDto::from).collect();
    Ok((StatusCode::OK, Json(dtos)))
}

pub async fn get_schema(
    Extension(_ctx): Extension<SecurityContext>,
    Extension(svc): Extension<Arc<CredentialsStorageService>>,
    Path(id): Path<Uuid>,
) -> ApiResult<impl IntoResponse> {
    let schema = svc.schemas.get(id).await?;
    Ok((StatusCode::OK, Json(SchemaDto::from(schema))))
}

pub async fn create_schema(
    Extension(_ctx): Extension<SecurityContext>,
    Extension(svc): Extension<Arc<CredentialsStorageService>>,
    Json(req): Json<CreateSchemaRequest>,
) -> ApiResult<impl IntoResponse> {
    let create = CreateSchema {
        id: req.id,
        name: req.name,
        schema: req.schema,
        fields_to_mask: req.fields_to_mask,
    };
    let created = svc.schemas.create(create).await?;
    Ok((StatusCode::CREATED, Json(SchemaDto::from(created))))
}

pub async fn update_schema(
    Extension(_ctx): Extension<SecurityContext>,
    Extension(svc): Extension<Arc<CredentialsStorageService>>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateSchemaRequest>,
) -> ApiResult<impl IntoResponse> {
    let updated = svc.schemas.update(id, req.into()).await?;
    Ok((StatusCode::OK, Json(SchemaDto::from(updated))))
}

pub async fn delete_schema(
    Extension(_ctx): Extension<SecurityContext>,
    Extension(svc): Extension<Arc<CredentialsStorageService>>,
    Path(id): Path<Uuid>,
) -> ApiResult<impl IntoResponse> {
    svc.schemas.delete(id).await?;
    Ok(no_content().into_response())
}

// ---------------------------------------------------------------------------
// Credential Definitions
// ---------------------------------------------------------------------------

pub async fn list_definitions(
    Extension(_ctx): Extension<SecurityContext>,
    Extension(svc): Extension<Arc<CredentialsStorageService>>,
) -> ApiResult<impl IntoResponse> {
    let defs = svc.definitions.list().await?;
    let dtos: Vec<CredentialDefinitionDto> = defs
        .into_iter()
        .map(CredentialDefinitionDto::from)
        .collect();
    Ok((StatusCode::OK, Json(dtos)))
}

pub async fn get_definition(
    Extension(_ctx): Extension<SecurityContext>,
    Extension(svc): Extension<Arc<CredentialsStorageService>>,
    Path(id): Path<Uuid>,
) -> ApiResult<impl IntoResponse> {
    let def = svc.definitions.get(id).await?;
    Ok((StatusCode::OK, Json(CredentialDefinitionDto::from(def))))
}

pub async fn create_definition(
    Extension(_ctx): Extension<SecurityContext>,
    Extension(svc): Extension<Arc<CredentialsStorageService>>,
    Json(req): Json<CreateCredentialDefinitionRequest>,
) -> ApiResult<impl IntoResponse> {
    let create = CreateCredentialDefinition {
        id: req.id,
        name: req.name,
        description: req.description,
        schema_id: req.schema_id,
        default_value: req.default_value,
        allowed_app_ids: req.allowed_app_ids.unwrap_or_default(),
    };
    let created = svc.definitions.create(create).await?;
    Ok((
        StatusCode::CREATED,
        Json(CredentialDefinitionDto::from(created)),
    ))
}

pub async fn update_definition(
    Extension(_ctx): Extension<SecurityContext>,
    Extension(svc): Extension<Arc<CredentialsStorageService>>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateCredentialDefinitionRequest>,
) -> ApiResult<impl IntoResponse> {
    let updated = svc.definitions.update(id, req.into()).await?;
    Ok((StatusCode::OK, Json(CredentialDefinitionDto::from(updated))))
}

pub async fn delete_definition(
    Extension(_ctx): Extension<SecurityContext>,
    Extension(svc): Extension<Arc<CredentialsStorageService>>,
    Path(id): Path<Uuid>,
) -> ApiResult<impl IntoResponse> {
    svc.definitions.delete(id).await?;
    Ok(no_content().into_response())
}

// ---------------------------------------------------------------------------
// Credentials
// ---------------------------------------------------------------------------

pub async fn create_credential(
    Extension(ctx): Extension<SecurityContext>,
    Extension(svc): Extension<Arc<CredentialsStorageService>>,
    Json(req): Json<CreateCredentialRequest>,
) -> ApiResult<impl IntoResponse> {
    let tenant_id = ctx.subject_tenant_id();
    let input = CreateCredential {
        id: req.id,
        definition_name: req.definition_name,
        value: req.value,
        propagate: req.propagate,
    };
    let created = svc.credentials.create_credential(tenant_id, input).await?;
    Ok((StatusCode::CREATED, Json(CredentialDto::from(created))))
}

pub async fn update_credential(
    Extension(ctx): Extension<SecurityContext>,
    Extension(svc): Extension<Arc<CredentialsStorageService>>,
    Path(definition_name): Path<String>,
    Json(req): Json<UpdateCredentialRequest>,
) -> ApiResult<impl IntoResponse> {
    let tenant_id = ctx.subject_tenant_id();
    let input = UpdateCredential {
        definition_name,
        tenant_id,
        value: req.value,
        propagate: req.propagate,
    };
    let updated = svc.credentials.update_credential(input).await?;
    Ok((StatusCode::OK, Json(CredentialDto::from(updated))))
}

pub async fn delete_credential(
    Extension(ctx): Extension<SecurityContext>,
    Extension(svc): Extension<Arc<CredentialsStorageService>>,
    Path(definition_name): Path<String>,
) -> ApiResult<impl IntoResponse> {
    let tenant_id = ctx.subject_tenant_id();
    svc.credentials
        .delete_credential(&definition_name, tenant_id)
        .await?;
    Ok(no_content().into_response())
}

pub async fn get_credential(
    Extension(ctx): Extension<SecurityContext>,
    Extension(svc): Extension<Arc<CredentialsStorageService>>,
    Path(definition_name): Path<String>,
) -> ApiResult<impl IntoResponse> {
    let tenant_id = ctx.subject_tenant_id();
    let value = svc
        .credentials
        .get_credential_for_app(&definition_name, tenant_id)
        .await?;
    Ok((StatusCode::OK, Json(value)))
}
