# Technical Design — Credentials Storage Plugin

- [ ] `p3` - **ID**: `cpt-pc-cs-design-credentials-storage`

<!-- toc -->

- [1. Architecture Overview](#1-architecture-overview)
  - [1.1 Architectural Vision](#11-architectural-vision)
  - [1.2 Architecture Drivers](#12-architecture-drivers)
  - [1.3 Architecture Layers](#13-architecture-layers)
- [2. Principles & Constraints](#2-principles--constraints)
  - [2.1 Design Principles](#21-design-principles)
  - [2.2 Constraints](#22-constraints)
- [3. Technical Architecture](#3-technical-architecture)
  - [3.1 Domain Model](#31-domain-model)
  - [3.2 Component Model](#32-component-model)
  - [3.3 API Contracts](#33-api-contracts)
  - [3.4 External Dependencies](#34-external-dependencies)
  - [3.5 Interactions & Sequences](#35-interactions--sequences)
  - [3.6 Database schemas & tables](#36-database-schemas--tables)
- [4. Additional context](#4-additional-context)

<!-- /toc -->

## 1. Architecture Overview

### 1.1 Architectural Vision

Credentials Storage is designed as a self-contained module (deployable as part of the CredStore gateway) with a layered hexagonal architecture that isolates domain logic from infrastructure concerns.
The architecture prioritizes security-by-default: every credential value is encrypted before reaching the persistence
layer, and access is enforced at multiple levels — AuthN-validated identity (via the CyberFabric AuthN Resolver) and
authorization decisions from the CyberFabric AuthZ Resolver, which combines RBAC permissions on credentials with an
ABAC policy evaluated over each credential's opaque `metadata` and other request attributes. The plugin itself
contains no caller-scoping logic; every access decision is delegated to AuthZ.

Stage 1 focuses strictly on encrypted storage and tenant-hierarchy propagation for credentials. Credential type
semantics (value shape, default values, cross-application sharing) are out of scope — each credential carries an opaque
GTS type URI that this plugin stores and returns without interpretation. Stage 2 will integrate with the Global Type
System (GTS, `https://github.com/GlobalTypeSystem/gts-spec`) for value validation and default-value resolution.


Tenant encryption key management is abstracted behind a `KeyProvider` port, allowing keys to be stored either locally
(in-database, for development and simple deployments) or in a separate, hardened key management service (for production
environments where cryptographic isolation is required). This separation ensures that compromising the credentials
database does not expose encryption keys, and vice versa.

### 1.2 Architecture Drivers

Requirements that significantly influence architecture decisions.

#### Functional Drivers

| Requirement                                                         | Design Response                                                                                      |
|---------------------------------------------------------------------|------------------------------------------------------------------------------------------------------|
| `cpt-pc-cs-fr-credential-encrypt` — Encrypt all values at rest      | Dedicated cryptography service with AES-256-GCM; per-tenant key management via pluggable KeyProvider  |
| `cpt-pc-cs-fr-credential-propagate` — Hierarchical propagation      | Credential merge logic in service layer resolves own → inherited chain up the tenant tree            |
| `cpt-pc-cs-fr-credential-decrypt-app` — Decrypted values for apps   | Service layer decrypts credentials for the owning application using the tenant's encryption key       |
| `cpt-pc-cs-fr-auth-authn` — AuthN Resolver authentication           | Axum AuthN middleware calls the CyberFabric AuthN Resolver; `SecurityContext` propagated through request context |
| `cpt-pc-cs-fr-auth-permission` — Permission checks                  | PEP handler calls the CyberFabric AuthZ Resolver (PDP) before write operations and applies returned constraints  |

#### NFR Allocation

| NFR ID                           | NFR Summary                        | Allocated To                                 | Design Response                                                                                        | Verification Approach                                                                    |
|----------------------------------|------------------------------------|----------------------------------------------|--------------------------------------------------------------------------------------------------------|------------------------------------------------------------------------------------------|
| `cpt-pc-cs-nfr-encryption`       | 100% encryption at rest            | Cryptography Service + KeyProvider           | All credential values pass through encrypt() before persistence; no direct DB writes bypass encryption | Integration tests verify no plaintext in DB                                              |
| `cpt-pc-cs-nfr-tenant-isolation` | Per-tenant cryptographic isolation | KeyProvider + Cryptography Service           | Each tenant has a unique AES-256 key; keys never shared across tenants; keys can be isolated in external KMS | Unit tests verify key uniqueness; integration tests verify cross-tenant decryption fails |
| `cpt-pc-cs-nfr-response-time`    | p95 ≤ 100ms at 100 concurrent      | All layers + KeyProvider cache               | Async I/O via Tokio; connection pooling; AuthN/AuthZ Resolver calls reused across requests; in-process per-tenant key cache (short TTL) absorbs hot paths so `ExternalKeyProvider` round-trips are not on every encrypt/decrypt | Load testing with k6 or similar; cache hit-rate surfaced in telemetry                    |


### 1.3 Architecture Layers

- [ ] `p3` - **ID**: `cpt-pc-cs-tech-layers`

```mermaid
graph TB
    GW["Module Gateway<br/>(HTTP termination)"]

    subgraph "Credentials Storage Plugin"
        SVC["Service Layer<br/>Business Logic + Crypto"]
        REPO["Repository Layer<br/>SQLx + Sea-Query"]
        DOM["Domain Layer<br/>Entities + Value Objects"]
        KP["KeyProvider Port"]
    end

    DB["DB"]
    AUTHN["AuthN Resolver"]
    AUTHZ["AuthZ Resolver"]
    EXT_KMS["External Key Service<br/>(Vault / KMS)"]

    GW -->|"authenticate"| AUTHN
    GW -->|"SecurityContext + request"| SVC
    SVC -->|"Queries"| REPO
    SVC -->|"get/create key"| KP
    SVC -.->|"Uses"| DOM
    REPO -->|"SQL"| DB
    KP -->|"Local mode"| DB
    KP -->|"External mode"| EXT_KMS
    SVC -->|"access evaluation"| AUTHZ
```

| Layer          | Responsibility                                                                      | Technology                     |
|----------------|-------------------------------------------------------------------------------------|--------------------------------|
| Service        | Business logic, credential merge (own → inherited), encryption/decryption           | Core Rust, AES-GCM, Serde JSON |
| Repository     | Data access, query construction, connection management                              | SQLx 0.8, Sea-Query 0.32       |
| KeyProvider    | Tenant key retrieval and creation; abstracts local DB vs external key service       | Trait-based port (see §3.2)    |
| Domain         | Core entities (Credential, TenantKey), value objects, enums                         | Pure Rust structs              |
| Infrastructure | Configuration, telemetry, server lifecycle, connection pooling                      | Tokio, OpenTelemetry           |

## 2. Principles & Constraints

### 2.1 Design Principles

#### Encryption by Default

- [ ] `p2` - **ID**: `cpt-pc-cs-principle-encryption-default`

All credential values are encrypted before leaving the service layer. The repository layer never receives plaintext
credential data. This ensures that even in the event of a database compromise or SQL injection, credential values remain
protected.

#### Tenant Isolation

- [ ] `p2` - **ID**: `cpt-pc-cs-principle-tenant-isolation`

Each tenant's credentials are encrypted with a unique per-tenant key. No shared encryption keys exist between tenants.
This provides cryptographic isolation — compromising one tenant's key does not expose another tenant's data.

#### Key–Data Separation

- [ ] `p1` - **ID**: `cpt-pc-cs-principle-key-data-separation`

Encryption keys and encrypted data MUST be separable into distinct security domains. The service abstracts key
management behind a `KeyProvider` port so that tenant keys can reside in a separate, hardened service (e.g., HashiCorp
Vault, AWS KMS, or a dedicated internal key management service) rather than in the same database as encrypted
credentials. This ensures that a single breach (database compromise, SQL injection, backup leak) does not expose both
ciphertext and the keys needed to decrypt it.

#### Least Privilege Access

- [ ] `p2` - **ID**: `cpt-pc-cs-principle-least-privilege`

Access control is enforced at multiple levels: the CyberFabric AuthN Resolver verifies identity and produces a
`SecurityContext`; the CyberFabric AuthZ Resolver returns the access decision based on RBAC permissions on credentials
combined with ABAC over each credential's `metadata` and other request attributes. The plugin performs no caller-scoping
itself — all visibility and write decisions are delegated to AuthZ.

#### Defense in Depth

- [ ] `p2` - **ID**: `cpt-pc-cs-principle-defense-in-depth`

Security is layered: network-level (transport and network policy as provided by the runtime), transport-level (TLS),
authentication (CyberFabric AuthN Resolver), authorization (CyberFabric AuthZ Resolver — RBAC plus ABAC over per-credential
`metadata`), and data-level (AES-256-GCM encryption). No single layer's failure exposes credentials.

#### Clean Architecture

- [ ] `p3` - **ID**: `cpt-pc-cs-principle-clean-architecture`

Domain entities have zero dependencies on infrastructure. The service layer orchestrates business logic without
knowledge of HTTP or SQL specifics. This separation enables unit testing of business logic without database or network
setup.

### 2.2 Constraints
#### Database Persistence

- [ ] `p2` - **ID**: `cpt-pc-cs-constraint-db`

All persistent data (credentials and, when `DatabaseKeyProvider` is active, tenant keys) must be stored in the
platform-provided database. CyberFabric is database-agnostic; the concrete engine is selected by platform configuration.
No alternative storage backends (e.g., object stores, in-memory caches) are permitted for primary persistence without a
new ADR.

#### Horizontal Scalability & Operability

- [ ] `p2` - **ID**: `cpt-pc-cs-constraint-scalability`

The module must be runnable as stateless, horizontally scalable instances behind a load balancer — with no in-process
state that prevents scale-out. Instances must expose readiness and liveness signals, support graceful shutdown (drain
in-flight requests before exit), and tolerate rolling updates without dropped requests. The concrete runtime environment
(Kubernetes, bare VMs, managed container platforms) is not prescribed; CyberFabric is environment-agnostic.

#### Authenticated Caller Required

- [ ] `p2` - **ID**: `cpt-pc-cs-constraint-authn`

All plugin operations require an authenticated caller. The plugin MUST NOT terminate HTTP or validate bearer tokens;
token validation is performed at the Module Gateway, which delegates to the CyberFabric AuthN Resolver and produces a
`SecurityContext`. The plugin consumes that `SecurityContext` supplied by the Module Gateway and propagates it through its internal service layer.
Token format (JWT, opaque, or other) is owned by the AuthN Resolver plugin and is not observable inside this plugin.

#### Multi-Tenant Hierarchy Support

- [ ] `p2` - **ID**: `cpt-pc-cs-constraint-multi-tenant`

The service must support a hierarchical tenant model for credential propagation. Credential
resolution must traverse the tenant tree from child to parent.

#### Cross-Tenant Key Access for Inheritance

- [ ] `p2` - **ID**: `cpt-pc-cs-constraint-key-access-scope`

Inherited credentials are encrypted with the **owning ancestor tenant's key**, not the requesting tenant's key. Walk-up
resolution therefore requires the plugin to decrypt under any tenant key it may encounter while traversing the tenant
tree. The plugin's identity at the KMS (`ExternalKeyProvider` mode) MUST be granted read access to **every tenant key in
the deployment** — typically via a single namespace/path (e.g., a Vault mount or KMS key alias prefix) covering all
tenant keys, not per-tenant least-privilege scoping. Restrictive per-tenant policies that deny access to ancestor keys
will silently break credential inheritance for descendants. Encrypt-time access is naturally limited to the requesting
tenant's key (because credentials are written under their owning tenant); the broad scope applies to read/decrypt.
Cross-tenant exposure is bounded inside the plugin by the AuthZ Resolver decision and the merge logic — the KMS is not
the authorization boundary here.

## 3. Technical Architecture

### 3.1 Domain Model

**Technology**: Rust structs with Serde serialization

**Core Entities**:

| Entity         | Description                                                                                                                                                                              |
|----------------|------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| Credential     | An encrypted tenant-specific credential value owned by an application, identified by name within that application, with propagation metadata and an opaque GTS type URI.                 |
| TenantKey      | A per-tenant AES-256-GCM encryption key used for credential encryption and decryption. Managed by the `KeyProvider` port — may reside in local DB or an external key management service.    |
| CredentialView | Read-model returned to callers after merge resolution. Carries the decrypted value, the resolution `origin` (own vs. inherited), the source tenant, and the opaque GTS type URI.         |

**Value objects & enums**:

- `CredentialOrigin` — enum with variants `Credential` (resolved on the requesting tenant) and `Inherited` (resolved on an ancestor tenant via propagation).

**`CredentialView` fields**:

| Field             | Type               | Description                                                                                                          |
|-------------------|--------------------|----------------------------------------------------------------------------------------------------------------------|
| name              | String             | Credential name as requested by the caller.                                                                          |
| value             | Plaintext bytes    | Decrypted credential value. Held in memory only for the duration of the response and never logged.                   |
| origin            | `CredentialOrigin` | Indicates whether the value came from the requesting tenant or from an ancestor via propagation.                     |
| owner_tenant_id   | UUID               | Tenant that owns the resolved credential. Equals the requesting tenant when `origin = Credential`; the ancestor tenant when `origin = Inherited`. |
| gts_type_uri      | String             | Opaque GTS type URI copied from the resolved credential; not interpreted by this module.                             |
| propagate         | bool               | Whether the resolved credential is configured to propagate to descendant tenants. Lets callers see the propagation chain is intact when `origin = Inherited`. |
| created           | Timestamp (UTC)    | Creation timestamp of the resolved credential row, copied from the `credentials.created` column.                     |

**Relationships**:

- TenantKey 1→N Credential: each tenant key encrypts all credentials for that tenant (key resolution via `KeyProvider`)
- CredentialView is derived from exactly one Credential at read time; it is not persisted.

**Notes on scope**:

Schema and credential-definition concerns (type/shape declaration, default values, application access control lists)
are out of scope for stage 1. Credential type information is represented as an opaque GTS type URI stored alongside each
credential; resolving, validating, or propagating that type is delegated to the Global Type System (see
`https://github.com/GlobalTypeSystem/gts-spec`) and is not performed by this module. Stage 2 will introduce GTS-backed
validation and default-value resolution.

### 3.2 Component Model

#### Services

- [ ] `p2` - **ID**: `cpt-pc-cs-component-services`

##### Why this component exists

Encapsulates all business logic including credential CRUD orchestration, encryption/decryption, credential
merge/propagation resolution.

##### Responsibility scope

Orchestrate credential lifecycle: encrypt values, persist via repository. Resolve credential merge from two sources
(own → inherited) by walking the tenant hierarchy. Obtain tenant encryption keys via `KeyProvider` (auto-generate on
first use). Perform cryptographic operations (AES-256-GCM encrypt/decrypt).

##### Responsibility boundaries

Does NOT validate credential values against their GTS type — the type URI is stored opaquely and validation is
deferred to stage 2 (delegated to GTS). Does NOT handle HTTP concerns (routing, status codes). Does NOT construct SQL
queries — delegates to repositories. Does NOT manage database connections.

##### Related components (by ID)

- `cpt-pc-cs-component-repositories` — delegates data persistence
- `cpt-pc-cs-component-key-provider` — obtains tenant encryption keys for crypto operations
- `cpt-pc-cs-component-domain` — uses domain entities for business operations

#### KeyProvider

- [ ] `p1` - **ID**: `cpt-pc-cs-component-key-provider`

##### Why this component exists

Decouples tenant key management from the credential storage service, enabling encryption keys to be stored in a
separate security domain from the encrypted data. This is a critical cybersecurity boundary: if the credentials database
is compromised, the attacker gains only ciphertext without the keys to decrypt it.

##### Responsibility scope

Provide a `KeyProvider` trait (async port) with two operations: `get_or_create_key(tenant_id) → TenantKey` and
`get_key(tenant_id) → Option<TenantKey>`. Two implementations:

1. **`DatabaseKeyProvider`** (default) — stores keys in the local `tenant_keys` table. Suitable for development,
   testing, and single-tenant deployments where operational simplicity is prioritized over key isolation.

2. **`ExternalKeyProvider`** — delegates key storage and retrieval to an external key management service
   (e.g., HashiCorp Vault Transit secrets engine, AWS KMS, Azure Key Vault, or a dedicated internal KMS).
   Suitable for production multi-tenant deployments where regulatory or security requirements demand that
   encryption keys are never co-located with encrypted data.

The active implementation is selected by configuration (`key_provider` field). The `ExternalKeyProvider` communicates
with the external service over mTLS and authenticates via service-specific credentials (Vault token, IAM role, etc.).

##### Idempotent key creation (concurrency contract)

`get_or_create_key(tenant_id)` MUST be safe under concurrent first-write for the same tenant. Two `create_credential`
calls arriving simultaneously for a tenant that has no `tenant_keys` row must converge on a single key — never produce
duplicate keys, fail with a constraint violation visible to the caller, or leave one credential encrypted under an
orphaned key. This is enforced at the `KeyProvider` trait contract; both implementations must satisfy it.

- **`DatabaseKeyProvider`** — implement get-or-create as a single statement using `INSERT ... ON CONFLICT (tenant_id)
  DO NOTHING RETURNING *`, followed (when `RETURNING` returns no row) by a `SELECT` to read the row created by the
  winning concurrent call. The `UNIQUE(tenant_id)` constraint on `tenant_keys` is the source of truth that resolves
  the race; the `ON CONFLICT` clause turns the loser's collision into a benign no-op rather than a propagated error.
  An advisory lock (`pg_advisory_xact_lock(hashtext('tenant_key:' || tenant_id))`) is an acceptable alternative when
  the underlying engine does not support `ON CONFLICT` semantics, but `ON CONFLICT` is preferred where available
  because it avoids serializing unrelated tenants' first-writes through a single lock.
- **`ExternalKeyProvider`** — the chosen KMS API call MUST be idempotent on `tenant_id` (e.g., Vault Transit's
  create-key is naturally idempotent — re-creating an existing named key is a no-op; AWS KMS requires the
  alias-then-create-then-bind pattern with conflict handling on the alias). If the underlying API is not idempotent,
  the implementation MUST serialize first-writes per tenant (e.g., via the same in-process single-flight used by the
  cache, scoped to the create path) and treat the "already exists" error from the KMS as success — fetching the
  existing key.
- **Cache interaction** — the in-process cache (below) MUST NOT be populated until the underlying provider has
  confirmed a single canonical row/key; otherwise a losing concurrent create could cache the wrong key id. Single-
  flight on `get_or_create_key` collapses concurrent misses into one provider call and resolves this naturally.

##### In-process key cache

Both implementations sit behind a thin in-process cache to keep the response-time NFR
(`cpt-pc-cs-nfr-response-time`, p95 ≤ 100ms) achievable when `ExternalKeyProvider` is active — without it, every
encrypt/decrypt would incur a network round-trip to the external KMS, which alone can exceed the budget under load.

| Aspect            | Decision                                                                                                     |
|-------------------|--------------------------------------------------------------------------------------------------------------|
| Scope             | Keyed by `tenant_id`. One entry caches the resolved `TenantKey` (the AES-256 material plus its `key_id`).    |
| Storage           | Per-process (each plugin instance has its own cache); never persisted to disk; zeroized on eviction/shutdown. |
| TTL               | Short — default 60s, configurable. Bounds blast radius if a key is rotated or revoked at the KMS.            |
| Negative caching  | Not cached — a missing key on read is a normal miss path that must always re-check the source of truth.      |
| Invalidation      | TTL expiry plus explicit invalidation on key-rotation events (stage 2). On any decrypt failure attributable to a stale key, the entry is dropped and the lookup retried once against the provider. |
| Concurrency       | Single-flight per `tenant_id`: concurrent misses for the same tenant collapse to one provider call.          |
| Bound             | LRU cap (default 10 000 entries) so a tenant explosion cannot grow the cache without bound.                  |

The cache is part of the `KeyProvider` port surface — both `DatabaseKeyProvider` and `ExternalKeyProvider` use the same
wrapper, so cache semantics do not vary by deployment mode. Cached material is treated as sensitive: not logged, not
included in telemetry, zeroized on drop.

##### Responsibility boundaries

Does NOT perform encryption/decryption — only manages key lifecycle (create, retrieve, future: rotate).
Does NOT contain business logic. Does NOT access credential data.

##### Related components (by ID)

- `cpt-pc-cs-component-services` — service layer calls KeyProvider to obtain keys for encrypt/decrypt operations
- `cpt-pc-cs-component-domain` — uses TenantKey domain entity

#### Repositories

- [ ] `p2` - **ID**: `cpt-pc-cs-component-repositories`

##### Why this component exists

Abstracts all database interactions, providing a clean data access interface to the service layer without exposing SQL
or database-specific concerns.

##### Responsibility scope

CRUD operations for credentials. Construct type-safe SQL queries via Sea-Query. Map database rows to domain entities.
Manage transactions where required.

##### Responsibility boundaries

Does NOT contain business logic. Does NOT perform encryption — receives already-encrypted data. Does NOT resolve or
validate GTS types — the `gts_type_uri` column is stored and returned opaquely. Does NOT manage tenant keys (delegated
to `KeyProvider`).

##### Related components (by ID)

- `cpt-pc-cs-component-services` — called by service layer for data access
- `cpt-pc-cs-component-domain` — maps DB rows to domain entities

#### Domain

- [ ] `p2` - **ID**: `cpt-pc-cs-component-domain`

##### Why this component exists

Defines core business entities and value objects with zero infrastructure dependencies, ensuring domain logic is
testable and portable.

##### Responsibility scope

Define Credential and TenantKey entities. Define CredentialOrigin enum (Credential, Inherited). Define CredentialView
for merged credential representations.

##### Responsibility boundaries

Does NOT depend on any infrastructure crate (no SQLx, no Axum, no HTTP). Does NOT contain persistence logic. Pure data
structures with business semantics.

##### Related components (by ID)

- `cpt-pc-cs-component-services` — services operate on domain entities
- `cpt-pc-cs-component-repositories` — repositories map to/from domain entities

#### Infrastructure

- [ ] `p3` - **ID**: `cpt-pc-cs-component-infrastructure`

##### Why this component exists

Manages cross-cutting concerns: application configuration, server lifecycle, telemetry, connection pooling, and
dependency wiring.

##### Responsibility scope

Load configuration from environment variables. Initialize database connection pool. Set up OpenTelemetry tracing and
metrics. Configure Axum router with all routes and middleware. Manage graceful server shutdown. Provide ApiState for
dependency injection across layers.

##### Responsibility boundaries

Does NOT contain business logic. Does NOT handle individual HTTP requests. Bootstraps the application and provides
shared infrastructure.

##### Related components (by ID)

- `cpt-pc-cs-component-services` — infrastructure creates service instances in ApiState

### 3.3 API Contracts

This plugin exposes no external API of its own. It implements the `CredStorePluginClientV1` trait defined in `credstore-sdk` and is invoked in-process by the parent CredStore gateway, which owns HTTP termination and the public REST surface. See the parent module's [DESIGN.md §4.3](../../docs/DESIGN.md#43-api-contracts) for the trait signature and REST contract.

### 3.4 External Dependencies

#### Database

| Aspect                | Details                                                                          |
|-----------------------|----------------------------------------------------------------------------------|
| Purpose               | Persistent storage for credentials. Tenant keys stored here only when `DatabaseKeyProvider` is active.                           |
| Protocol              | TCP/SQL via SQLx async driver                                                    |
| Authentication        | Username/password from environment configuration                                 |
| Connection Management | Connection pool via `db-utils`; configurable pool size                           |

#### External Key Management Service (optional)

| Aspect         | Details                                                                       |
|----------------|-------------------------------------------------------------------------------|
| Purpose        | Tenant encryption key storage and lifecycle when `ExternalKeyProvider` is active. Provides key–data separation for production security posture. |
| Protocol       | HTTPS/mTLS (Vault HTTP API, AWS KMS API, or custom REST/gRPC)                |
| Authentication | Service-specific: Vault token, cloud IAM role, runtime service identity        |
| Access Scope   | The plugin's KMS identity MUST be authorized to read **every tenant key** in the deployment (typically a single mount/alias-prefix covering all tenants). Required because inheritance decrypts credentials under ancestor-tenant keys — see `cpt-pc-cs-constraint-key-access-scope`. Per-tenant least-privilege policies will break propagation. |
| Caching        | The plugin maintains an in-process per-tenant key cache (short TTL, default 60s) — see `cpt-pc-cs-component-key-provider`. Required to meet the response-time NFR; without it, each encrypt/decrypt incurs a KMS round-trip. |
| Error Handling | Key Service unavailable blocks all encrypt/decrypt operations once cache entries expire; readiness signal reflects KMS connectivity |

#### CyberFabric AuthZ Resolver

| Aspect         | Details                                                                                      |
|----------------|----------------------------------------------------------------------------------------------|
| Purpose        | Authorization (PDP) — returns the decision and optional query-level constraints for `Credential.Manage` on write operations |
| Protocol       | In-process plugin call or out-of-process gRPC (AuthZEN-style request/response)               |
| Authentication | Same-process trust in-process; mTLS for out-of-process deployments                           |
| Error Handling | Deny decision is returned to the caller as a permission error; resolver unavailable blocks write operations |

> **Note on authentication**: Bearer-token validation is not a dependency of this plugin. Token validation is owned by the Module Gateway, which calls the CyberFabric AuthN Resolver and supplies the resulting `SecurityContext` to the plugin. The `SecurityContext` shape contract is captured in PRD §7.2 (`cpt-pc-cs-contract-authn`).

### 3.5 Interactions & Sequences

#### Create Credential with Encryption

**ID**: `cpt-pc-cs-seq-create-credential`

**Use cases**: `cpt-pc-cs-usecase-admin-manage-creds`

**Actors**: `cpt-pc-cs-actor-tenant-admin`

```mermaid
sequenceDiagram
    actor Admin as Tenant Admin
    participant GW as Module Gateway
    participant AuthN as AuthN Resolver
    participant Svc as Credentials Service
    participant AuthZ as AuthZ Resolver (PDP)
    participant KP as KeyProvider
    participant Crypto as Cryptography Service
    participant Repo as Credentials Repository
    participant DB as Database
    participant EXT as External KMS (optional)

    Admin->>GW: POST /credentials (Bearer + body)
    GW->>AuthN: authenticate(bearer_token)
    AuthN-->>GW: SecurityContext
    GW->>Svc: create_credential(tenant_id, name, gts_type_uri, value, propagate, metadata, SecurityContext)
    Svc->>AuthZ: access evaluation (write permission, metadata)
    AuthZ-->>Svc: decision = permit
    Svc->>KP: get_or_create_key(tenant_id)
    Note over KP: single-flight per tenant_id<br/>collapses concurrent misses
    alt DatabaseKeyProvider (local mode)
        KP->>DB: INSERT ... ON CONFLICT (tenant_id) DO NOTHING RETURNING *
        alt row returned (winner)
            DB-->>KP: newly created tenant_key
        else no row (loser — concurrent create won)
            KP->>DB: SELECT tenant_key WHERE tenant_id = ?
            DB-->>KP: existing tenant_key
        end
    else ExternalKeyProvider (external mode)
        KP->>EXT: idempotent get-or-create key for tenant_id
        EXT-->>KP: tenant_key (existing or freshly created)
    end
    KP-->>Svc: tenant_key
    Svc->>Crypto: encrypt(value, tenant_key)
    Crypto-->>Svc: encrypted_value
    Svc->>Repo: insert_credential(tenant_id, name, gts_type_uri, encrypted, propagate, metadata)
    Repo->>DB: INSERT credential
    DB-->>Repo: OK
    Svc-->>GW: Credential created
    GW-->>Admin: 201 Created
```

**Description**: Administrator creates a credential. The Module Gateway terminates HTTP and delegates token validation
to the AuthN Resolver, which produces a `SecurityContext`; the Gateway then invokes the plugin with that context and the
caller-supplied `metadata`. The plugin's Credentials Service obtains the access decision from the AuthZ Resolver (PDP) —
RBAC for the credential write permission combined with ABAC over the supplied `metadata` — then retrieves the tenant's
encryption key via the `KeyProvider` port (either from local DB or an external key management service), encrypts the
value, and persists it together with its opaque `gts_type_uri` and `metadata`. Type validation against the GTS type is
deferred to stage 2.

#### Retrieve Credential with Merge Resolution

**ID**: `cpt-pc-cs-seq-retrieve-credential`

**Use cases**: `cpt-pc-cs-usecase-app-retrieve-cred`, `cpt-pc-cs-usecase-credential-inheritance`

**Actors**: `cpt-pc-cs-actor-vendor-app`

```mermaid
sequenceDiagram
    actor App as Vendor App
    participant GW as Module Gateway
    participant AuthN as AuthN Resolver
    participant Svc as Credentials Service
    participant AuthZ as AuthZ Resolver (PDP)
    participant KP as KeyProvider
    participant Crypto as Cryptography Service
    participant Repo as Credentials Repository
    participant DB as Database

    App->>GW: GET /credentials/{name} (Bearer)
    GW->>AuthN: authenticate(bearer_token)
    AuthN-->>GW: SecurityContext
    GW->>Svc: get_credential(tenant_id, name, SecurityContext)
    Svc->>Repo: get_credential(tenant_id, name)
    Repo->>DB: SELECT credential WHERE tenant_id AND name
    DB-->>Repo: credential or NULL

    alt Credential found for tenant
        Svc->>Svc: origin = "Credential"
    else No credential — walk up tenant ancestry
        Svc->>Repo: get_propagated_credential(ancestor_tenant_ids, name)
        Repo->>DB: SELECT credential WHERE tenant_id IN ancestors AND name AND propagate=true
        DB-->>Repo: inherited credential or NULL
        alt Inherited credential found
            Svc->>Svc: origin = "Inherited"
        else No inherited
            Svc-->>GW: 404 Not Found
        end
    end

    Svc->>AuthZ: access evaluation (read permission, credential.metadata)
    AuthZ-->>Svc: decision = permit | deny
    alt deny
        Svc-->>GW: 404 Not Found
    end

    Svc->>KP: get_key(credential.tenant_id)
    KP-->>Svc: tenant_key
    Svc->>Crypto: decrypt(encrypted_value, tenant_key)
    Crypto-->>Svc: decrypted_value
    Svc-->>GW: CredentialView(decrypted, origin, gts_type_uri)
    GW-->>App: 200 OK (value + origin + gts_type_uri)
```

**Description**: Application retrieves a credential. The Module Gateway terminates HTTP and supplies the plugin with a
`SecurityContext` produced by the AuthN Resolver. The service resolves the credential through the two-source merge chain
(own → inherited by walking the tenant ancestry for credentials with `propagate=true`), then asks the AuthZ Resolver to
evaluate the read decision against the caller and the credential's `metadata` (RBAC plus ABAC). If denied, the response
is **404 Not Found** — identical to the "no credential resolved" path — to prevent the caller from distinguishing
"exists but you cannot see it" from "does not exist", which would otherwise enable enumeration of credential names
(per PRD FR-002, UC-002 alt-flow, and Acceptance #4). If permitted, the value is decrypted with the owning tenant's key and returned together with its origin and
opaque `gts_type_uri`. If no credential is found at either level the response is 404 — default-value fallback is
deferred to stage 2 (delegated to GTS).

### 3.6 Database schemas & tables

- [ ] `p3` - **ID**: `cpt-pc-cs-db-main`

#### Table: credentials

**ID**: `cpt-pc-cs-dbtable-credentials`

| Column          | Type         | Description                                                                                              |
|-----------------|--------------|----------------------------------------------------------------------------------------------------------|
| id              | UUID         | Primary key                                                                                              |
| tenant_id       | UUID         | Tenant that owns this credential                                                                         |
| name            | VARCHAR(255) | Credential name (unique per tenant, case-insensitive)                                                    |
| gts_type_uri    | TEXT         | Opaque GTS type URI describing the credential value's type (not interpreted or validated by this module) |
| encrypted_value | BYTEA        | AES-256-GCM encrypted credential value (nonce prepended)                                                 |
| propagate       | BOOLEAN      | Whether this credential propagates to child tenants                                                      |
| metadata        | JSONB        | Opaque metadata supplied at write time and consumed by the AuthZ ABAC policy; not interpreted by this plugin |
| key_id          | UUID         | Foreign key to tenant_keys table (encryption key used)                                                   |
| created         | TIMESTAMPTZ  | Creation timestamp                                                                                       |

**PK**: `id`
**Constraints**: UNIQUE(tenant_id, name), FK(key_id → tenant_keys.id), NOT NULL(tenant_id, name, gts_type_uri,
encrypted_value, key_id)

#### Table: tenant_keys

**ID**: `cpt-pc-cs-dbtable-tenant-keys`

| Column    | Type        | Description                                              |
|-----------|-------------|----------------------------------------------------------|
| id        | UUID        | Primary key                                              |
| tenant_id | UUID        | Tenant this key belongs to (unique — one key per tenant) |
| key       | VARCHAR(64) | Base64-encoded 32-byte AES-256 encryption key            |
| created   | TIMESTAMPTZ | Key creation timestamp                                   |

**PK**: `id`
**Constraints**: UNIQUE(tenant_id), NOT NULL(tenant_id, key)

**Additional info**: This table is used only by the `DatabaseKeyProvider` implementation. When the `ExternalKeyProvider`
is active, this table is not used — keys are stored and managed by the external key management service. Tenant keys are
auto-generated when the first credential is created for a tenant.

**Security note**: In production multi-tenant deployments, the `ExternalKeyProvider` is strongly recommended. Storing
encryption keys in the same database as encrypted credentials means a single database compromise exposes both ciphertext
and keys. See `cpt-pc-cs-principle-key-data-separation`.

## 4. Additional context

- Stage 1 scope excludes schema and credential-definition management. Credential type information is stored as an
  opaque GTS type URI (`gts_type_uri` column); this module does not resolve, validate, or interpret it. Stage 2 will
  introduce GTS-backed validation and default-value resolution by reusing the Global Type System
  (`https://github.com/GlobalTypeSystem/gts-spec`), which is already the platform's type system for plugin registration
  (see `modules/credstore/docs/DESIGN.md`).
- Caller-level access scoping is delegated entirely to the CyberFabric AuthZ Resolver. The plugin defines RBAC
  permissions on the credential resource and exposes each credential's `metadata` for evaluation by an ABAC policy;
  the plugin itself contains no caller-scoping logic. Cross-tenant or cross-caller sharing models are realized as
  AuthZ policies on `metadata` and require no schema changes here.
- Encryption key storage in the application database (`DatabaseKeyProvider`) is suitable for development and
  single-tenant deployments. For production multi-tenant environments, the `ExternalKeyProvider` delegates key
  management to a separate service (HashiCorp Vault, AWS KMS, etc.) to achieve key–data separation.
  See `cpt-pc-cs-principle-key-data-separation` and `cpt-pc-cs-component-key-provider`.
- Key rotation is not yet implemented. The `KeyProvider` abstraction is designed to accommodate future key rotation
  support — the external KMS can manage key versions while the service re-encrypts credentials on rotation events.