use serde::Deserialize;
use uuid::Uuid;

/// Plugin configuration.
#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct CredentialsStoragePluginConfig {
    /// Vendor name for GTS instance registration.
    pub vendor: String,

    /// Plugin priority (lower = higher priority).
    pub priority: i16,

    /// Default application ID to use for schemas/definitions when no explicit app context.
    ///
    /// Use the nil UUID (`00000000-0000-0000-0000-000000000000`) to match
    /// records created without explicit application scoping.
    pub application_id: Uuid,
}

impl Default for CredentialsStoragePluginConfig {
    fn default() -> Self {
        Self {
            vendor: "hyperspot".to_owned(),
            priority: 100,
            application_id: Uuid::nil(),
        }
    }
}
