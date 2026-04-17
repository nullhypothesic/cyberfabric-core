# PRD — Credentials Storage Plugin


<!-- toc -->

- [1. Overview](#1-overview)
  - [1.1 Purpose](#11-purpose)
  - [1.2 Background / Problem Statement](#12-background--problem-statement)
  - [1.3 Goals (Business Outcomes)](#13-goals-business-outcomes)
  - [1.4 Glossary](#14-glossary)
- [2. Actors](#2-actors)
  - [2.1 Human Actors](#21-human-actors)
  - [2.2 System Actors](#22-system-actors)
- [3. Operational Concept & Environment](#3-operational-concept--environment)
  - [3.1 Module-Specific Environment Constraints](#31-module-specific-environment-constraints)
- [4. Scope](#4-scope)
  - [4.1 In Scope](#41-in-scope)
  - [4.2 Out of Scope](#42-out-of-scope)
- [5. Functional Requirements](#5-functional-requirements)
  - [5.1 P1 — Encryption & Key Management](#51-p1--encryption--key-management)
  - [5.2 P1 — Credential Lifecycle](#52-p1--credential-lifecycle)
- [6. Non-Functional Requirements](#6-non-functional-requirements)
  - [6.1 Module-Specific NFRs](#61-module-specific-nfrs)
- [7. Public Library Interfaces](#7-public-library-interfaces)
  - [7.1 Public API Surface](#71-public-api-surface)
  - [7.2 External Integration Contracts](#72-external-integration-contracts)
- [8. Use Cases](#8-use-cases)
- [9. Acceptance Criteria](#9-acceptance-criteria)
- [10. Dependencies](#10-dependencies)
- [11. Assumptions](#11-assumptions)
- [12. Risks](#12-risks)
- [13. Open Questions](#13-open-questions)
- [14. Traceability](#14-traceability)

<!-- /toc -->

<!--
=============================================================================
PRODUCT REQUIREMENTS DOCUMENT (PRD)
=============================================================================
PURPOSE: Define WHAT the system must do and WHY — business requirements,
functional capabilities, and quality attributes.

SCOPE:
  ✓ Business goals and success criteria
  ✓ Actors (users, systems) that interact with this module
  ✓ Functional requirements (WHAT, not HOW)
  ✓ Non-functional requirements (quality attributes, SLOs)
  ✓ Scope boundaries (in/out of scope)
  ✓ Assumptions, dependencies, risks

NOT IN THIS DOCUMENT (see other templates):
  ✗ Stakeholder needs (managed at project/task level by steering committee)
  ✗ Technical architecture, design decisions → DESIGN.md
  ✗ Why a specific technical approach was chosen → ADR/
  ✗ Detailed implementation flows, algorithms → features/

STANDARDS ALIGNMENT:
  - IEEE 830 / ISO/IEC/IEEE 29148:2018 (requirements specification)
  - IEEE 1233 (system requirements)
  - ISO/IEC 15288 / 12207 (requirements definition)

REQUIREMENT LANGUAGE:
  - Use "MUST" or "SHALL" for mandatory requirements (implicit default)
  - Do not use "SHOULD" or "MAY" — use priority p2/p3 instead
  - Be specific and clear; no fluff, bloat, duplication, or emoji
=============================================================================
-->

## 1. Overview

### 1.1 Purpose

The Credentials Storage Plugin is a backend plugin for the CredStore gateway module that provides encrypted credential storage with schema-driven validation, field-level masking, credential merge/propagation resolution, and pluggable tenant key management. It replaces reliance on external credential backends with a self-contained service that manages the full credential lifecycle internally.

### 1.2 Background / Problem Statement

The CredStore gateway supports multiple backend plugins for secret persistence. Existing plugins (VendorA Credstore, OS keychain) delegate encryption and storage to external systems, which limits control over encryption strategy, schema validation, and credential propagation logic.

Production multi-tenant deployments require per-tenant cryptographic isolation, defense-in-depth key management, and the ability to separate encryption keys from encrypted data. Existing backends do not natively support schema-driven credential validation, field-level masking for user-facing responses, or application-level access control lists. The Credentials Storage Plugin addresses these gaps by providing a self-contained credential management service with built-in encryption, schema validation, and tenant-aware credential resolution.


### 1.3 Goals (Business Outcomes)

- Encrypt all credentials before storing them in the database so that no secret is ever persisted in plaintext; each tenant's data is cryptographically isolated from other tenants
- Enable schema-driven credential validation so credential structure is enforced at creation time, reducing runtime errors from malformed credentials by 100%
- Support credential propagation across tenant hierarchies so child tenants inherit parent credentials without manual duplication
- Store encryption keys in the same database by default; support external key services via pluggable KeyProvider for production deployments requiring key–data separation

### 1.4 Glossary

| Term | Definition |
|------|------------|
| Schema | A JSON Schema definition that describes the structure of credential values and which fields to mask |
| Credential Definition | A named configuration that links a schema to a specific application, provides default credential values, and specifies which applications are allowed to access credentials of this type |
| Credential | A tenant-specific encrypted credential value associated with a credential definition |
| Tenant Key | A per-tenant encryption key used for credential encryption and decryption |
| KeyProvider | An abstraction for tenant key management — implementations may store keys locally or delegate to an external service |
| Credential Propagation | The process of resolving a credential value through the tenant hierarchy (own → inherited → default) |
| Field-Level Masking | Replacing sensitive fields in a credential value with masked placeholders in user-facing responses |

## 2. Actors

### 2.1 Human Actors

#### Tenant Admin

**ID**: `cpt-pc-cs-actor-tenant-admin`

**Role**: Authenticated administrator managing credentials, credential definitions, and schemas for their tenant. Creates, updates, and deletes credentials. Configures credential definitions with default values and application access control.
**Needs**: CRUD operations on schemas, credential definitions, and credentials within their tenant namespace. Ability to control which applications can access specific credentials.

### 2.2 System Actors

#### Vendor Application

**ID**: `cpt-pc-cs-actor-vendor-app`

**Role**: Platform application that retrieves decrypted credential values at runtime. Identified by `application_id` from JWT claims. Access is restricted to credential definitions that include the application in their `allowed_app_ids` list.
**Needs**: Retrieve decrypted credential values for authorized credential definitions. Credential resolution must traverse the tenant hierarchy transparently.

## 3. Operational Concept & Environment

### 3.1 Module-Specific Environment Constraints

- The plugin requires a database for persistent storage of schemas, credential definitions, credentials, and tenant keys (when local key storage is active)
- The plugin requires access to a JWT issuer (Vendor IDP) for authentication
- The plugin requires access to the Permission Service for write-operation authorization
- When external key management is active, the plugin requires network access to the external key service

## 4. Scope

### 4.1 In Scope

- Schema CRUD — create, list, get, update, delete (App)
- Credential definition CRUD with schema binding, default values, and application access control (App)
- Credential read with field-level masking (App, Admin)
- Credential write — create, update, delete with encryption at rest (Admin)
- Credential merge/propagation resolution — own → inherited → default (App, Admin)
- Per-tenant encryption key management via pluggable KeyProvider
- JWT-based authentication for all API endpoints (App, Admin)
- Permission-based authorization for credential write operations (Admin)
- Application-level access control — allowed_app_ids on credential definitions (App)

### 4.2 Out of Scope

- Pluggable external key providers for key–data separation (planned; KeyProvider abstraction accommodates it)
- Encryption key rotation (planned future capability; KeyProvider abstraction accommodates it)
- User-scoped credentials (personal secrets per user, similar to Google Colab secrets)
- Secret versioning or history
- Automatic credential expiration or rotation
- Cross-tenant credential transfer
- Gateway-level concerns (plugin selection, hierarchical walk-up for simple backends, SDK traits)

## 5. Functional Requirements

### 5.1 P1 — Encryption & Key Management

#### Credential Value Encryption

- [ ] `p1` - **ID**: `cpt-pc-cs-fr-credential-encrypt`

<!-- cpt-cf-id-content -->
The system **MUST** encrypt all credential values before persistence. No plaintext credential data **MUST** reach the persistence layer.

**Rationale**: Defense-in-depth — even if the database is compromised or SQL injection occurs, credential values remain protected.
**Actors**: `cpt-pc-cs-actor-tenant-admin`, `cpt-pc-cs-actor-vendor-app`
<!-- cpt-cf-id-content -->

#### JWT Authentication

- [ ] `p1` - **ID**: `cpt-pc-cs-fr-auth-jwt`

<!-- cpt-cf-id-content -->
All API endpoints **MUST** require JWT Bearer token authentication. Tokens **MUST** be validated against JWKS endpoints provided by the Vendor IDP. Identity claims (tenant_id, application_id) **MUST** be extracted and propagated to the service layer.

**Rationale**: Ensures all API access is authenticated and tenant/application identity is available for authorization and scoping decisions.
**Actors**: `cpt-pc-cs-actor-tenant-admin`, `cpt-pc-cs-actor-vendor-app`
<!-- cpt-cf-id-content -->

#### Permission-Based Authorization

- [ ] `p1` - **ID**: `cpt-pc-cs-fr-auth-permission`

<!-- cpt-cf-id-content -->
The system **MUST** use the CyberFabric authZ (authorization) Permission Service to resolve and validate the `Credential.Manage` permission before allowing write operations (create, update, delete) on schemas, credential definitions, and credentials.

**Rationale**: Enforces least-privilege access control for mutating operations.
**Actors**: `cpt-pc-cs-actor-tenant-admin`
<!-- cpt-cf-id-content -->

### 5.2 P1 — Credential Lifecycle

#### Credential Propagation

- [ ] `p1` - **ID**: `cpt-pc-cs-fr-credential-propagate`

<!-- cpt-cf-id-content -->
The system **MUST** resolve credential values through the tenant hierarchy using a three-source merge chain: (1) the tenant's own credential, (2) an inherited credential from a parent tenant (where propagation is enabled), (3) the credential definition's default value. The first available source in this order **MUST** be returned. The response **MUST** indicate the origin of the resolved value.

**Rationale**: Enables parent tenants to share credentials with child tenants without manual duplication, while allowing child tenants to override with their own values.
**Actors**: `cpt-pc-cs-actor-vendor-app`
<!-- cpt-cf-id-content -->

#### Field-Level Masking

- [ ] `p1` - **ID**: `cpt-pc-cs-fr-credential-mask`

<!-- cpt-cf-id-content -->
The system **MUST** apply field-level masking to credential values in user-facing responses. Masked fields **MUST** be determined by the schema's `fields_to_mask` configuration. Pre-computed masked values **MUST** be stored alongside encrypted values.

**Rationale**: Tenant admins need to see credential structure and metadata without exposing sensitive field values.
**Actors**: `cpt-pc-cs-actor-tenant-admin`
<!-- cpt-cf-id-content -->

#### Decrypted Values for Applications

- [ ] `p1` - **ID**: `cpt-pc-cs-fr-credential-decrypt-app`

<!-- cpt-cf-id-content -->
The system **MUST** return decrypted credential values to authorized applications. Application identity **MUST** be determined from the JWT `application_id` claim. The response path for applications **MUST** differ from the user-facing response path (which returns masked values).

**Rationale**: Applications need the actual credential values to authenticate with external services.
**Actors**: `cpt-pc-cs-actor-vendor-app`
<!-- cpt-cf-id-content -->

#### Application Access Control

- [ ] `p1` - **ID**: `cpt-pc-cs-fr-definition-allowed-apps`

<!-- cpt-cf-id-content -->
The system **MUST** enforce application-level access control on credential retrieval. Each credential definition **MUST** specify an `allowed_app_ids` list. Only the owning application and applications in this list **MUST** be permitted to retrieve decrypted credential values. Unauthorized applications **MUST** receive a not-found response.

**Rationale**: Prevents unauthorized applications from accessing credentials outside their scope.
**Actors**: `cpt-pc-cs-actor-vendor-app`
<!-- cpt-cf-id-content -->

## 6. Non-Functional Requirements

### 6.1 Module-Specific NFRs

#### Encryption at Rest

- [ ] `p1` - **ID**: `cpt-pc-cs-nfr-encryption`

<!-- cpt-cf-id-content -->
100% of credential values **MUST** be encrypted before persistence. No plaintext credential data **MUST** exist in the database at any time.

**Threshold**: Zero plaintext credential values in the database
**Rationale**: Regulatory and enterprise security requirements demand encryption at rest for all sensitive data.
**Architecture Allocation**: See DESIGN.md for implementation approach
<!-- cpt-cf-id-content -->

#### Per-Tenant Cryptographic Isolation

- [ ] `p1` - **ID**: `cpt-pc-cs-nfr-tenant-isolation`

<!-- cpt-cf-id-content -->
Each tenant's credentials **MUST** be encrypted with a unique per-tenant key. No encryption keys **MUST** be shared between tenants. Compromising one tenant's key **MUST NOT** expose another tenant's data.

**Threshold**: Zero shared encryption keys between tenants; cross-tenant decryption attempts fail 100% of the time
**Rationale**: Multi-tenant security requires cryptographic isolation to prevent cross-tenant data exposure.
**Architecture Allocation**: See DESIGN.md for implementation approach
<!-- cpt-cf-id-content -->

#### Response Time

- [ ] `p2` - **ID**: `cpt-pc-cs-nfr-response-time`

<!-- cpt-cf-id-content -->
API responses **MUST** complete within 100ms at p95 under 100 concurrent requests.

**Threshold**: p95 latency ≤ 100ms at 100 concurrent requests
**Rationale**: Credential retrieval is on the critical path for application startup and API call execution.
**Architecture Allocation**: See DESIGN.md for implementation approach
<!-- cpt-cf-id-content -->

## 7. Public Library Interfaces

### 7.1 Public API Surface

#### REST API

See `cpt-pc-cs-interface-rest-api` (defined in DESIGN.md).

- **Type**: REST API
- **Stability**: stable
- **Description**: REST/JSON API for schema CRUD, credential definition CRUD, and credential CRUD with encryption and masking. All endpoints require JWT authentication.
- **Breaking Change Policy**: Versioned URL path (`/api/credentials-storage/v1/`); backward-compatible within major version

### 7.2 External Integration Contracts

#### JWT Authentication Contract

- [ ] `p1` - **ID**: `cpt-pc-cs-contract-jwt-auth`

- **Direction**: required from client (inbound JWT from Vendor IDP)
- **Protocol/Format**: HTTPS for JWKS endpoint retrieval; JWT Bearer token on all API requests
- **Compatibility**: Standard JWKS/JWT; IDP changes to signing keys propagate via JWKS refresh

#### Permission Service Contract

- [ ] `p1` - **ID**: `cpt-pc-cs-contract-permission-check`

- **Direction**: required from client (outbound to Permission Service)
- **Protocol/Format**: HTTP/REST call to platform Permission Service API
- **Compatibility**: Service-to-service via internal Kubernetes network; Permission Service API changes require plugin update

#### Tenant Hierarchy Contract

- [ ] `p1` - **ID**: `cpt-pc-cs-contract-tenant-hierarchy`

- **Direction**: required from client (tenant hierarchy information for credential propagation)
- **Protocol/Format**: Platform API for tenant parent resolution
- **Compatibility**: Tenant hierarchy API changes require plugin update

## 8. Use Cases

#### UC-001: Admin Manages Credentials

- [ ] `p1` - **ID**: `cpt-pc-cs-usecase-admin-manage-creds`

**Actor**: `cpt-pc-cs-actor-tenant-admin`

**Preconditions**:
- Admin is authenticated with a valid JWT containing tenant_id
- Admin has `Credential.Manage` permission

**Main Flow**:
1. Admin creates a schema defining the credential structure and masked fields
2. Admin creates a credential definition binding the schema to an application, with default values and allowed_app_ids
3. Admin creates a credential providing a value that validates against the schema
4. System encrypts the value, generates a masked version, and persists both
5. Admin retrieves the credential — system returns the masked value

**Postconditions**:
- Credential is stored encrypted. Admin sees only masked values.

**Alternative Flows**:
- **Schema validation fails**: System rejects the credential with a validation error
- **Credential already exists**: System updates the existing credential value

#### UC-002: Application Retrieves Credential

- [ ] `p1` - **ID**: `cpt-pc-cs-usecase-app-retrieve-cred`

**Actor**: `cpt-pc-cs-actor-vendor-app`

**Preconditions**:
- Application is authenticated with a valid JWT containing application_id and tenant_id
- Application is in the credential definition's allowed_app_ids list (or is the owning application)

**Main Flow**:
1. Application requests a credential by definition name
2. System verifies application authorization against allowed_app_ids
3. System retrieves the tenant's credential (own → inherited → default)
4. System decrypts the value using the tenant's encryption key
5. System returns the decrypted value with origin metadata

**Postconditions**:
- Application has the decrypted credential value for use with external services.

**Alternative Flows**:
- **Application not authorized**: System returns not-found (prevents enumeration)
- **No credential exists**: System falls back to inherited or default value
- **No value at any level**: System returns not-found

#### UC-003: Credential Inheritance Through Hierarchy

- [ ] `p1` - **ID**: `cpt-pc-cs-usecase-credential-inheritance`

**Actor**: `cpt-pc-cs-actor-vendor-app`

**Preconditions**:
- Parent tenant has a credential with propagation enabled
- Child tenant has no own credential for the same definition
- Application is authorized for the credential definition

**Main Flow**:
1. Application requests a credential for the child tenant
2. System finds no own credential for the child tenant
3. System resolves the inherited credential from the parent tenant (propagation enabled)
4. System decrypts using the parent tenant's encryption key
5. System returns the value with origin indicating "Inherited"

**Postconditions**:
- Child tenant uses the parent's credential without manual duplication.

**Alternative Flows**:
- **Propagation disabled on parent's credential**: System falls back to the credential definition's default value
- **Child tenant creates own credential**: Own credential takes precedence over inherited on next retrieval

## 9. Acceptance Criteria

- [ ] All credential values are encrypted before persistence — zero plaintext in the database
- [ ] Each tenant has a unique encryption key — cross-tenant decryption fails
- [ ] Schema validation rejects credentials that do not match the defined JSON Schema
- [ ] Field-level masking correctly replaces sensitive fields in user-facing responses
- [ ] Applications receive decrypted values; non-application callers receive masked values
- [ ] Application access control enforces allowed_app_ids — unauthorized apps receive not-found
- [ ] Credential propagation resolves through the three-source merge chain (own → inherited → default)
- [ ] All API endpoints require JWT authentication
- [ ] Write operations require `Credential.Manage` permission

## 10. Dependencies

| Dependency | Description | Criticality |
|------------|-------------|-------------|
| CredStore Gateway | Parent module that routes requests to this plugin via `CredStorePluginClientV1` trait | `p1` |
| Vendor IDP | JWT issuer for authentication — provides JWKS endpoints for token validation | `p1` |
| Permission Service | Authorization — validates `Credential.Manage` permission for write operations | `p1` |
| Platform Tenant API | Tenant hierarchy information for credential propagation resolution | `p1` |
| Database (PostgreSQL initially) | Persistent storage for schemas, definitions, credentials, and tenant keys | `p1` |
| External Key Service | External key management service (Vault, KMS) for tenant key storage when external KeyProvider is active | `p2` |

## 11. Assumptions

- The CredStore gateway selects this plugin at runtime via GTS configuration; only one storage plugin is active per deployment
- JWT tokens from Vendor IDP contain `tenant_id` and `application_id` claims
- Permission Service is reachable over internal Kubernetes network
- Tenant hierarchy information is available via platform API
- Tenant encryption keys are auto-generated on first credential creation for a tenant
- The default KeyProvider stores keys in the same database as credentials; an external KeyProvider is available for production deployments requiring key–data separation

## 12. Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| External key service unavailability | All encrypt/decrypt operations blocked when external KeyProvider is active | High-availability key service deployment; readiness probe reflects KMS connectivity; key caching with short TTL |
| Keys co-located with encrypted data (local KeyProvider) | Single breach exposes both ciphertext and keys | Use external KeyProvider in production multi-tenant deployments; restrict local KeyProvider to development/test |
| Schema evolution breaks existing credentials | Existing credentials fail validation against updated schema | Schema versioning strategy; backward-compatible schema changes only |
| Permission Service unavailability | Write operations blocked | Circuit breaker pattern; readiness probe reflects Permission Service connectivity |
| Credential propagation depth at deep nesting | Increased latency for credential resolution | Early termination on first resolved value; cache tenant hierarchy queries |

## 13. Open Questions

- **Key rotation strategy**: When and how are tenant encryption keys rotated? The KeyProvider abstraction accommodates rotation, but the rotation workflow (re-encryption of existing credentials, key versioning) is not yet defined.
- **User-scoped credentials**: Should the system support personal secrets per user (similar to Google Colab secrets)? The current data model may need extensions.
- **Credential definition versioning**: Should credential definitions support versioning to track changes over time?

## 14. Traceability

- **Parent PRD**: [modules/credstore/docs/PRD.md](../../docs/PRD.md)
- **Parent Design**: [modules/credstore/docs/DESIGN.md](../../docs/DESIGN.md)
- **Design**: [DESIGN.md](./DESIGN.md)
- **ADRs**: ADR/ (planned)
- **Features**: features/ (planned)
