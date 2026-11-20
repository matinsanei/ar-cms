use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

// ========================
// CORE CMS SCHEMA TYPES
// ========================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributeDefinition {
    #[serde(rename = "type")]
    pub field_type: String,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub unique: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "minLength")]
    pub min_length: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "maxLength")]
    pub max_length: Option<u32>,
    #[serde(flatten, default)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentTypeInfo {
    #[serde(rename = "singularName")]
    pub singular_name: String,
    #[serde(rename = "pluralName")]
    pub plural_name: String,
    #[serde(rename = "displayName")]
    pub display_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentTypeSchema {
    pub uid: String,
    pub kind: String, // "collectionType" or "singleType"
    pub info: ContentTypeInfo,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Value>,
    pub attributes: HashMap<String, AttributeDefinition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentEntry {
    pub id: u64,
    #[serde(rename = "documentId")]
    pub document_id: String,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    #[serde(rename = "publishedAt")]
    pub published_at: Option<String>,
    #[serde(default = "default_locale_en")]
    pub locale: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(flatten)]
    pub fields: HashMap<String, Value>,
}

pub fn default_locale_en() -> Option<String> {
    Some("en".to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedEntries {
    pub uid: String,
    pub entries: Vec<ContentEntry>,
}

pub fn entry_filename(uid: &str) -> String {
    let clean = uid.replace("::", "_").replace(':', "_").replace('/', "_");
    format!("{}.json", clean)
}

// ========================
// ADMIN SECURITY & RBAC TYPES
// ========================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminRole {
    pub id: u64,
    pub name: String,
    pub code: String,
    pub description: String,
    #[serde(rename = "usersCount", default)]
    pub users_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminUser {
    pub id: u64,
    #[serde(rename = "documentId")]
    pub document_id: String,
    pub firstname: String,
    pub lastname: String,
    pub username: String,
    pub email: String,
    #[serde(rename = "isActive", default = "default_true")]
    pub is_active: bool,
    #[serde(default)]
    pub blocked: bool,
    #[serde(rename = "preferedLanguage", default)]
    pub prefered_language: Option<String>,
    pub roles: Vec<AdminRole>,
    #[serde(skip_serializing, default)]
    pub password_hash: String,
}

pub fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Webhook {
    pub id: String,
    pub name: String,
    pub url: String,
    pub headers: HashMap<String, String>,
    pub events: Vec<String>,
    #[serde(rename = "isEnabled", default = "default_true")]
    pub is_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiToken {
    pub id: u64,
    pub name: String,
    pub description: String,
    #[serde(rename = "type")]
    pub token_type: String, // "full-access", "read-only", "custom"
    #[serde(rename = "accessKey")]
    pub access_key: String,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    #[serde(rename = "lastUsedAt")]
    pub last_used_at: Option<String>,
    #[serde(rename = "lifespan", skip_serializing_if = "Option::is_none")]
    pub lifespan: Option<Value>,
    #[serde(rename = "expiresAt", skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
    pub permissions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PersistedTokens {
    pub api_tokens: Vec<ApiToken>,
    pub admin_tokens: Vec<ApiToken>,
    pub transfer_tokens: Vec<ApiToken>,
}

// ========================
// MEDIA LIBRARY ASSET
// ========================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaAsset {
    pub id: u64,
    #[serde(rename = "documentId")]
    pub document_id: String,
    pub name: String,
    #[serde(rename = "alternativeText", skip_serializing_if = "Option::is_none")]
    pub alternative_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub caption: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub formats: Option<Value>,
    pub hash: String,
    pub ext: String,
    pub mime: String,
    pub size: f64,
    pub url: String,
    #[serde(rename = "previewUrl", skip_serializing_if = "Option::is_none")]
    pub preview_url: Option<String>,
    pub provider: String,
    #[serde(rename = "provider_metadata", skip_serializing_if = "Option::is_none")]
    pub provider_metadata: Option<Value>,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
}

// ========================
// AUDIT LOG & REVISIONS
// ========================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentRevision {
    pub id: u64,
    pub uid: String,
    #[serde(rename = "documentId")]
    pub document_id: String,
    pub version: u32,
    pub status: String,
    #[serde(rename = "authorEmail", skip_serializing_if = "Option::is_none")]
    pub author_email: Option<String>,
    pub snapshot: Value,
    #[serde(rename = "createdAt")]
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogEntry {
    pub id: u64,
    #[serde(rename = "userId", skip_serializing_if = "Option::is_none")]
    pub user_id: Option<u64>,
    #[serde(rename = "userEmail", skip_serializing_if = "Option::is_none")]
    pub user_email: Option<String>,
    pub action: String,
    #[serde(rename = "targetType", skip_serializing_if = "Option::is_none")]
    pub target_type: Option<String>,
    #[serde(rename = "targetId", skip_serializing_if = "Option::is_none")]
    pub target_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ip: Option<String>,
    #[serde(rename = "createdAt")]
    pub created_at: String,
}
