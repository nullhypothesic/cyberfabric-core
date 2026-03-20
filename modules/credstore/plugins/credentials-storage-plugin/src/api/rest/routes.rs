use std::sync::Arc;

use axum::http::StatusCode;
use axum::{Extension, Router};
use modkit::api::operation_builder::LicenseFeature;
use modkit::api::{OpenApiRegistry, OperationBuilder};
use uuid::Uuid;

use crate::domain::service::CredentialsStorageService;

use super::dto::{
    CreateCredentialDefinitionRequest, CreateCredentialRequest, CreateSchemaRequest,
    CredentialDefinitionDto, CredentialDto, SchemaDto, UpdateCredentialDefinitionRequest,
    UpdateCredentialRequest, UpdateSchemaRequest,
};
use super::handlers;

struct License;

impl AsRef<str> for License {
    fn as_ref(&self) -> &'static str {
        "gts.x.core.lic.feat.v1~x.core.global.base.v1"
    }
}

impl LicenseFeature for License {}

pub fn register_routes(
    mut router: Router,
    openapi: &dyn OpenApiRegistry,
    service: Arc<CredentialsStorageService>,
    _app_id: Uuid,
) -> Router {
    // ------------------------------------------------------------------
    // Schemas
    // ------------------------------------------------------------------
    router = OperationBuilder::get("/credentials-storage/v1/schemas")
        .operation_id("credentials_storage.list_schemas")
        .summary("List schemas")
        .tag("Credentials Storage")
        .authenticated()
        .require_license_features::<License>([])
        .handler(handlers::list_schemas)
        .json_response_with_schema::<Vec<SchemaDto>>(openapi, StatusCode::OK, "List of schemas")
        .standard_errors(openapi)
        .register(router, openapi);

    router = OperationBuilder::post("/credentials-storage/v1/schemas")
        .operation_id("credentials_storage.create_schema")
        .summary("Create schema")
        .tag("Credentials Storage")
        .authenticated()
        .require_license_features::<License>([])
        .json_request::<CreateSchemaRequest>(openapi, "Schema data")
        .handler(handlers::create_schema)
        .json_response_with_schema::<SchemaDto>(openapi, StatusCode::CREATED, "Schema created")
        .standard_errors(openapi)
        .register(router, openapi);

    router = OperationBuilder::get("/credentials-storage/v1/schemas/{id}")
        .operation_id("credentials_storage.get_schema")
        .summary("Get schema by ID")
        .tag("Credentials Storage")
        .authenticated()
        .require_license_features::<License>([])
        .handler(handlers::get_schema)
        .json_response_with_schema::<SchemaDto>(openapi, StatusCode::OK, "Schema")
        .standard_errors(openapi)
        .register(router, openapi);

    router = OperationBuilder::put("/credentials-storage/v1/schemas/{id}")
        .operation_id("credentials_storage.update_schema")
        .summary("Update schema")
        .tag("Credentials Storage")
        .authenticated()
        .require_license_features::<License>([])
        .json_request::<UpdateSchemaRequest>(openapi, "Schema update data")
        .handler(handlers::update_schema)
        .json_response_with_schema::<SchemaDto>(openapi, StatusCode::OK, "Schema updated")
        .standard_errors(openapi)
        .register(router, openapi);

    router = OperationBuilder::delete("/credentials-storage/v1/schemas/{id}")
        .operation_id("credentials_storage.delete_schema")
        .summary("Delete schema")
        .tag("Credentials Storage")
        .authenticated()
        .require_license_features::<License>([])
        .handler(handlers::delete_schema)
        .json_response(StatusCode::NO_CONTENT, "Schema deleted")
        .standard_errors(openapi)
        .register(router, openapi);

    // ------------------------------------------------------------------
    // Credential Definitions
    // ------------------------------------------------------------------
    router = OperationBuilder::get("/credentials-storage/v1/credential-definitions")
        .operation_id("credentials_storage.list_definitions")
        .summary("List credential definitions")
        .tag("Credentials Storage")
        .authenticated()
        .require_license_features::<License>([])
        .handler(handlers::list_definitions)
        .json_response_with_schema::<Vec<CredentialDefinitionDto>>(
            openapi,
            StatusCode::OK,
            "List of credential definitions",
        )
        .standard_errors(openapi)
        .register(router, openapi);

    router = OperationBuilder::post("/credentials-storage/v1/credential-definitions")
        .operation_id("credentials_storage.create_definition")
        .summary("Create credential definition")
        .tag("Credentials Storage")
        .authenticated()
        .require_license_features::<License>([])
        .json_request::<CreateCredentialDefinitionRequest>(openapi, "Credential definition data")
        .handler(handlers::create_definition)
        .json_response_with_schema::<CredentialDefinitionDto>(
            openapi,
            StatusCode::CREATED,
            "Credential definition created",
        )
        .standard_errors(openapi)
        .register(router, openapi);

    router = OperationBuilder::get("/credentials-storage/v1/credential-definitions/{id}")
        .operation_id("credentials_storage.get_definition")
        .summary("Get credential definition by ID")
        .tag("Credentials Storage")
        .authenticated()
        .require_license_features::<License>([])
        .handler(handlers::get_definition)
        .json_response_with_schema::<CredentialDefinitionDto>(
            openapi,
            StatusCode::OK,
            "Credential definition",
        )
        .standard_errors(openapi)
        .register(router, openapi);

    router = OperationBuilder::put("/credentials-storage/v1/credential-definitions/{id}")
        .operation_id("credentials_storage.update_definition")
        .summary("Update credential definition")
        .tag("Credentials Storage")
        .authenticated()
        .require_license_features::<License>([])
        .json_request::<UpdateCredentialDefinitionRequest>(
            openapi,
            "Credential definition update data",
        )
        .handler(handlers::update_definition)
        .json_response_with_schema::<CredentialDefinitionDto>(
            openapi,
            StatusCode::OK,
            "Credential definition updated",
        )
        .standard_errors(openapi)
        .register(router, openapi);

    router = OperationBuilder::delete("/credentials-storage/v1/credential-definitions/{id}")
        .operation_id("credentials_storage.delete_definition")
        .summary("Delete credential definition")
        .tag("Credentials Storage")
        .authenticated()
        .require_license_features::<License>([])
        .handler(handlers::delete_definition)
        .json_response(StatusCode::NO_CONTENT, "Credential definition deleted")
        .standard_errors(openapi)
        .register(router, openapi);

    // ------------------------------------------------------------------
    // Credentials
    // ------------------------------------------------------------------
    router = OperationBuilder::post("/credentials-storage/v1/credentials")
        .operation_id("credentials_storage.create_credential")
        .summary("Create credential")
        .tag("Credentials Storage")
        .authenticated()
        .require_license_features::<License>([])
        .json_request::<CreateCredentialRequest>(openapi, "Credential data")
        .handler(handlers::create_credential)
        .json_response_with_schema::<CredentialDto>(
            openapi,
            StatusCode::CREATED,
            "Credential created",
        )
        .standard_errors(openapi)
        .register(router, openapi);

    router = OperationBuilder::put("/credentials-storage/v1/credentials/{definition_name}")
        .operation_id("credentials_storage.update_credential")
        .summary("Update credential")
        .tag("Credentials Storage")
        .authenticated()
        .require_license_features::<License>([])
        .json_request::<UpdateCredentialRequest>(openapi, "Credential update data")
        .handler(handlers::update_credential)
        .json_response_with_schema::<CredentialDto>(
            openapi,
            StatusCode::OK,
            "Credential updated",
        )
        .standard_errors(openapi)
        .register(router, openapi);

    router = OperationBuilder::delete("/credentials-storage/v1/credentials/{definition_name}")
        .operation_id("credentials_storage.delete_credential")
        .summary("Delete credential")
        .tag("Credentials Storage")
        .authenticated()
        .require_license_features::<License>([])
        .handler(handlers::delete_credential)
        .json_response(StatusCode::NO_CONTENT, "Credential deleted")
        .standard_errors(openapi)
        .register(router, openapi);

    router = OperationBuilder::get("/credentials-storage/v1/credentials/{definition_name}")
        .operation_id("credentials_storage.get_credential")
        .summary("Get decrypted credential value by definition name")
        .tag("Credentials Storage")
        .authenticated()
        .require_license_features::<License>([])
        .handler(handlers::get_credential)
        .json_response_with_schema::<serde_json::Value>(
            openapi,
            StatusCode::OK,
            "Decrypted credential value",
        )
        .standard_errors(openapi)
        .register(router, openapi);

    router = router.layer(Extension(service));

    router
}
