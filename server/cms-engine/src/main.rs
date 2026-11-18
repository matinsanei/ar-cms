use axum::{
    extract::{Path, Query, State},
    http::{header, Method, StatusCode},
    response::{IntoResponse, Json},
    routing::{delete, get, post, put},
    Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs;
use std::path::{Path as StdPath, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::{Any, CorsLayer};
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

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

fn default_locale_en() -> Option<String> {
    Some("en".to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedEntries {
    pub uid: String,
    pub entries: Vec<ContentEntry>,
}

fn entry_filename(uid: &str) -> String {
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

fn default_true() -> bool {
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
// APPLICATION STATE
// ========================

pub struct AppState {
    pub schemas_dir: PathBuf,
    pub data_dir: PathBuf,
    pub schemas: RwLock<HashMap<String, ContentTypeSchema>>,
    pub entries: RwLock<HashMap<String, Vec<ContentEntry>>>,
    pub single_entries: RwLock<HashMap<String, HashMap<String, Value>>>,
    pub admins: RwLock<Vec<AdminUser>>,
    pub roles: RwLock<Vec<AdminRole>>,
    pub webhooks: RwLock<Vec<Webhook>>,
    pub api_tokens: RwLock<Vec<ApiToken>>,
    pub admin_tokens: RwLock<Vec<ApiToken>>,
    pub transfer_tokens: RwLock<Vec<ApiToken>>,
}

impl AppState {
    pub fn new(schemas_dir: PathBuf, data_dir: PathBuf) -> Self {
        let mut schemas = HashMap::new();
        if schemas_dir.exists() {
            if let Ok(dir_entries) = fs::read_dir(&schemas_dir) {
                for entry in dir_entries.flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|e| e.to_str()) == Some("json") {
                        if let Ok(content) = fs::read_to_string(&path) {
                            if let Ok(schema) = serde_json::from_str::<ContentTypeSchema>(&content) {
                                info!("Loaded schema on boot: {}", schema.uid);
                                schemas.insert(schema.uid.clone(), schema);
                            }
                        }
                    }
                }
            }
        }

        // Load admins if persisted
        let admins_file = data_dir.join("admins.json");
        let admins = if admins_file.exists() {
            fs::read_to_string(&admins_file)
                .ok()
                .and_then(|c| serde_json::from_str::<Vec<AdminUser>>(&c).ok())
                .unwrap_or_default()
        } else {
            Vec::new()
        };

        // Load webhooks
        let webhooks_file = data_dir.join("webhooks.json");
        let webhooks = if webhooks_file.exists() {
            fs::read_to_string(&webhooks_file)
                .ok()
                .and_then(|c| serde_json::from_str::<Vec<Webhook>>(&c).ok())
                .unwrap_or_default()
        } else {
            Vec::new()
        };

        // Default Roles
        let roles = vec![
            AdminRole {
                id: 1,
                name: "Super Admin".to_string(),
                code: "strapi-super-admin".to_string(),
                description: "Super Administrators can access and manage all features and settings.".to_string(),
                users_count: admins.len() as u64,
            },
            AdminRole {
                id: 2,
                name: "Editor".to_string(),
                code: "strapi-editor".to_string(),
                description: "Editors can create, edit and publish content.".to_string(),
                users_count: 0,
            },
            AdminRole {
                id: 3,
                name: "Author".to_string(),
                code: "strapi-author".to_string(),
                description: "Authors can create and edit their own content.".to_string(),
                users_count: 0,
            },
        ];

        // Load entries
        let entries_dir = data_dir.join("entries");
        let _ = fs::create_dir_all(&entries_dir);
        let mut initial_entries = HashMap::new();
        if let Ok(dir_entries) = fs::read_dir(&entries_dir) {
            for entry in dir_entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("json") {
                    if let Ok(content) = fs::read_to_string(&path) {
                        if let Ok(pe) = serde_json::from_str::<PersistedEntries>(&content) {
                            info!("Loaded {} persisted entries for {}", pe.entries.len(), pe.uid);
                            initial_entries.insert(pe.uid, pe.entries);
                        } else if let Ok(entries_list) = serde_json::from_str::<Vec<ContentEntry>>(&content) {
                            let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                            let uid = stem.replace('_', "::");
                            initial_entries.insert(uid, entries_list);
                        }
                    }
                }
            }
        }

        // Load single entries
        let singles_file = data_dir.join("singles.json");
        let single_entries = if singles_file.exists() {
            fs::read_to_string(&singles_file)
                .ok()
                .and_then(|c| serde_json::from_str::<HashMap<String, HashMap<String, Value>>>(&c).ok())
                .unwrap_or_default()
        } else {
            HashMap::new()
        };

        // Load persisted tokens
        let tokens_file = data_dir.join("tokens.json");
        let (api_tokens, admin_tokens, transfer_tokens) = if tokens_file.exists() {
            if let Ok(c) = fs::read_to_string(&tokens_file) {
                if let Ok(pt) = serde_json::from_str::<PersistedTokens>(&c) {
                    (pt.api_tokens, pt.admin_tokens, pt.transfer_tokens)
                } else {
                    (Vec::new(), Vec::new(), Vec::new())
                }
            } else {
                (Vec::new(), Vec::new(), Vec::new())
            }
        } else {
            (Vec::new(), Vec::new(), Vec::new())
        };

        Self {
            schemas_dir,
            data_dir,
            schemas: RwLock::new(schemas),
            entries: RwLock::new(initial_entries),
            single_entries: RwLock::new(single_entries),
            admins: RwLock::new(admins),
            roles: RwLock::new(roles),
            webhooks: RwLock::new(webhooks),
            api_tokens: RwLock::new(api_tokens),
            admin_tokens: RwLock::new(admin_tokens),
            transfer_tokens: RwLock::new(transfer_tokens),
        }
    }

    pub async fn persist_schema(&self, schema: &ContentTypeSchema) -> anyhow::Result<()> {
        let file_name = format!("{}.json", schema.info.singular_name);
        let path = self.schemas_dir.join(file_name);
        let formatted = serde_json::to_string_pretty(schema)?;
        fs::write(&path, formatted)?;
        info!("Persisted schema to file: {:?}", path);
        Ok(())
    }

    pub async fn save_tokens(&self) -> anyhow::Result<()> {
        let api = self.api_tokens.read().await;
        let admin = self.admin_tokens.read().await;
        let transfer = self.transfer_tokens.read().await;
        let pt = PersistedTokens {
            api_tokens: api.clone(),
            admin_tokens: admin.clone(),
            transfer_tokens: transfer.clone(),
        };
        let json = serde_json::to_string_pretty(&pt)?;
        let tokens_file = self.data_dir.join("tokens.json");
        fs::write(tokens_file, json)?;
        Ok(())
    }

    pub async fn save_entries(&self, uid: &str) -> anyhow::Result<()> {
        let entries_map = self.entries.read().await;
        let entries = entries_map.get(uid).cloned().unwrap_or_default();
        let entries_dir = self.data_dir.join("entries");
        let _ = fs::create_dir_all(&entries_dir);
        let file_path = entries_dir.join(entry_filename(uid));
        let pe = PersistedEntries {
            uid: uid.to_string(),
            entries,
        };
        let json = serde_json::to_string_pretty(&pe)?;
        fs::write(file_path, json)?;
        Ok(())
    }

    pub async fn save_singles(&self) -> anyhow::Result<()> {
        let singles = self.single_entries.read().await;
        let file_path = self.data_dir.join("singles.json");
        let json = serde_json::to_string_pretty(&*singles)?;
        fs::write(file_path, json)?;
        Ok(())
    }

    pub async fn save_admins(&self) -> anyhow::Result<()> {
        let admins = self.admins.read().await;
        let file_path = self.data_dir.join("admins.json");
        let json = serde_json::to_string_pretty(&*admins)?;
        fs::write(file_path, json)?;
        Ok(())
    }

    pub async fn save_webhooks(&self) -> anyhow::Result<()> {
        let webhooks = self.webhooks.read().await;
        let file_path = self.data_dir.join("webhooks.json");
        let json = serde_json::to_string_pretty(&*webhooks)?;
        fs::write(file_path, json)?;
        Ok(())
    }
}

// ========================
// MAIN ENTRY POINT
// ========================

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).ok();

    let schemas_dir = if let Ok(custom) = std::env::var("CMS_SCHEMAS_DIR") {
        PathBuf::from(custom)
    } else if StdPath::new("platform/apps/cms/server/schemas").exists() {
        StdPath::new("platform/apps/cms/server/schemas").canonicalize()?
    } else if StdPath::new("../schemas").exists() {
        StdPath::new("../schemas").canonicalize()?
    } else if StdPath::new("schemas").exists() {
        StdPath::new("schemas").canonicalize()?
    } else {
        PathBuf::from("schemas")
    };
    fs::create_dir_all(&schemas_dir)?;

    let data_dir = if let Ok(custom) = std::env::var("CMS_DATA_DIR") {
        PathBuf::from(custom)
    } else if StdPath::new("platform/apps/cms/server/data").exists() {
        StdPath::new("platform/apps/cms/server/data").canonicalize()?
    } else if StdPath::new("server/data").exists() {
        StdPath::new("server/data").canonicalize()?
    } else if StdPath::new("../data").exists() {
        StdPath::new("../data").canonicalize()?
    } else if StdPath::new("data").exists() {
        StdPath::new("data").canonicalize()?
    } else {
        let p = PathBuf::from("platform/apps/cms/server/data");
        let _ = fs::create_dir_all(&p);
        p
    };
    fs::create_dir_all(&data_dir)?;

    info!("Zenith CMS Unified Runtime Engine starting...");
    info!("Schemas storage directory: {:?}", schemas_dir);
    info!("Data persistence directory: {:?}", data_dir);

    let state = Arc::new(AppState::new(schemas_dir, data_dir));

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(vec![Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers(vec![header::CONTENT_TYPE, header::AUTHORIZATION]);

    let app = Router::new()
        // --- Admin Authentication & Setup Protocols ---
        .route("/admin/init", get(admin_init))
        .route("/admin/project-type", get(admin_project_type))
        .route("/admin/project-settings", get(admin_project_settings).post(admin_update_project_settings))
        .route("/admin/information", get(admin_information))
        .route("/admin/telemetry-properties", get(admin_telemetry))
        .route("/admin/plugins", get(admin_plugins))
        .route("/admin/plugins/", get(admin_plugins))
        .route("/admin/license-limit-information", get(admin_license_limit_information))
        .route("/admin/license-trial-time-left", get(admin_license_trial_time_left))
        .route("/admin/guided-tour-meta", get(admin_guided_tour_meta))
        .route("/admin/registration-info", get(admin_registration_info))
        .route("/admin/register-admin", post(admin_register_admin))
        .route("/admin/register", post(admin_register_user))
        .route("/admin/login", post(admin_login))
        .route("/admin/logout", post(admin_logout))
        .route("/admin/renew-token", post(admin_renew_token))
        .route("/admin/forgot-password", post(admin_forgot_password))
        .route("/admin/reset-password", post(admin_reset_password))
        .route("/admin/access-token", post(admin_access_token))
        .route("/admin/debug-dump", get(admin_debug_dump))
        // --- SSO & Providers ---
        .route("/admin/providers", get(admin_providers))
        .route("/admin/providers/", get(admin_providers))
        .route("/admin/providers/isSSOLocked", get(admin_providers_is_sso_locked))
        .route("/admin/providers/options", get(admin_providers_options).put(admin_providers_options_update))
        // --- RBAC & Permissions Check ---
        .route("/admin/permissions/check", post(admin_permissions_check))
        .route("/admin/permissions", get(admin_get_all_permissions))
        .route("/admin/permissions/", get(admin_get_all_permissions))
        // --- Admin Users & Permissions Management ---
        .route("/admin/users", get(admin_list_users).post(admin_create_user))
        .route("/admin/users/", get(admin_list_users).post(admin_create_user))
        .route("/admin/users/:id", get(admin_get_user).put(admin_update_user).delete(admin_delete_user))
        .route("/admin/users/batch-delete", post(admin_batch_delete_users))
        .route("/admin/users/me", get(admin_users_me).put(admin_users_me_update))
        .route("/admin/users/me/permissions", get(admin_users_me_permissions))
        .route("/admin/users/me/sessions", get(admin_sessions_list).delete(admin_sessions_revoke_all))
        .route("/admin/roles", get(admin_list_roles).post(admin_create_role))
        .route("/admin/roles/", get(admin_list_roles).post(admin_create_role))
        .route("/admin/roles/:id", get(admin_get_role).put(admin_update_role).delete(admin_delete_role))
        .route("/admin/roles/:id/permissions", get(admin_get_role_permissions).put(admin_update_role_permissions))
        // --- Webhooks Management ---
        .route("/admin/webhooks", get(admin_list_webhooks).post(admin_create_webhook))
        .route("/admin/webhooks/", get(admin_list_webhooks).post(admin_create_webhook))
        .route("/admin/webhooks/:id", get(admin_get_webhook).put(admin_update_webhook).delete(admin_delete_webhook))
        .route("/admin/webhooks/batch-delete", post(admin_batch_delete_webhooks))
        .route("/admin/webhooks/:id/trigger", post(admin_trigger_webhook))
        // --- API & Admin Tokens ---
        .route("/admin/api-tokens", get(admin_list_api_tokens).post(admin_create_api_token))
        .route("/admin/api-tokens/", get(admin_list_api_tokens).post(admin_create_api_token))
        .route("/admin/api-tokens/:id", get(admin_get_api_token).put(admin_update_api_token).delete(admin_delete_api_token))
        .route("/admin/api-tokens/:id/regenerate", post(admin_regenerate_api_token))
        .route("/admin/admin-tokens", get(admin_list_admin_tokens).post(admin_create_admin_token))
        .route("/admin/admin-tokens/", get(admin_list_admin_tokens).post(admin_create_admin_token))
        .route("/admin/admin-tokens/:id", get(admin_get_admin_token).put(admin_update_admin_token).delete(admin_delete_admin_token))
        .route("/admin/admin-tokens/:id/regenerate", post(admin_regenerate_admin_token))
        // --- Transfer Tokens ---
        .route("/admin/transfer/tokens", get(admin_list_transfer_tokens).post(admin_create_transfer_token))
        .route("/admin/transfer/tokens/", get(admin_list_transfer_tokens).post(admin_create_transfer_token))
        .route("/admin/transfer/tokens/:id", get(admin_get_transfer_token).put(admin_update_transfer_token).delete(admin_delete_transfer_token))
        .route("/admin/transfer/tokens/:id/regenerate", post(admin_regenerate_transfer_token))
        .route("/admin/transfer-tokens", get(admin_list_transfer_tokens).post(admin_create_transfer_token))
        .route("/admin/transfer-tokens/", get(admin_list_transfer_tokens).post(admin_create_transfer_token))
        .route("/admin/transfer-tokens/:id", get(admin_get_transfer_token).put(admin_update_transfer_token).delete(admin_delete_transfer_token))
        .route("/admin/transfer-tokens/:id/regenerate", post(admin_regenerate_transfer_token))
        // --- Homepage & Widgets ---
        .route("/admin/homepage/key-statistics", get(admin_homepage_key_statistics))
        .route("/admin/homepage/recent-documents", get(admin_homepage_recent_documents))
        .route("/admin/homepage/count-documents", get(admin_homepage_count_documents))
        .route("/admin/homepage/layout", get(admin_homepage_get_layout).put(admin_homepage_update_layout))
        // --- Content Manager Homepage Count ---
        .route("/content-manager/homepage/count-documents", get(content_manager_count_documents))
        // --- Upload & Media Library ---
        .route("/upload/settings", get(upload_get_settings).put(upload_update_settings))
        .route("/upload/configuration", get(upload_get_configuration))
        .route("/upload/folders", get(upload_get_folders))
        .route("/upload/folders/", get(upload_get_folders))
        .route("/upload/files", get(upload_list_files))
        .route("/upload/files/", get(upload_list_files))
        .route("/upload", post(upload_file))
        // --- i18n Locales ---
        .route("/i18n/locales", get(i18n_get_locales).post(i18n_create_locale))
        .route("/i18n/locales/", get(i18n_get_locales).post(i18n_create_locale))
        .route("/i18n/iso-locales", get(i18n_get_iso_locales))
        // --- Content-Type Builder Endpoints ---
        .route("/content-type-builder/schema", get(get_all_schemas))
        .route("/content-type-builder/reserved-names", get(get_reserved_names))
        .route("/content-type-builder/content-types", post(create_or_update_content_type))
        .route("/content-type-builder/content-types/:uid", get(get_content_type_by_uid).put(update_content_type_by_uid).delete(delete_content_type))
        .route("/content-type-builder/components", get(ctb_get_components))
        .route("/content-type-builder/components/:uid", get(ctb_get_component_by_uid))
        .route("/content-type-builder/update-schema", post(update_schema_batch))
        .route("/content-type-builder/update-schema-status", get(get_update_schema_status))
        // --- Content Manager Endpoints ---
        .route("/content-manager/init", get(content_manager_init))
        .route("/content-manager/uid/generate", post(content_manager_uid_generate))
        .route("/content-manager/uid/check-availability", post(content_manager_uid_check_availability))
        .route("/content-manager/components", get(content_manager_get_components))
        .route("/content-manager/components/", get(content_manager_get_components))
        .route("/content-manager/components/:uid/configuration", get(content_manager_get_component_configuration).put(content_manager_update_component_configuration))
        .route("/content-manager/content-types", get(get_all_content_types))
        .route("/content-manager/content-types/", get(get_all_content_types))
        .route("/content-manager/content-types-settings", get(get_all_content_types_settings))
        .route("/content-manager/content-types-settings/", get(get_all_content_types_settings))
        .route("/content-manager/content-types/:uid/configuration", get(get_content_type_configuration).put(update_content_type_configuration))
        .route("/content-manager/collection-types/:uid", get(get_entries).post(create_entry))
        .route("/content-manager/collection-types/:uid/:id", get(get_entry).put(update_entry).delete(delete_entry))
        .route("/content-manager/collection-types/:uid/:id/actions/countDraftRelations", get(content_manager_count_draft_relations))
        .route("/content-manager/collection-types/:uid/actions/countDraftRelations", get(content_manager_count_draft_relations))
        .route("/content-manager/single-types/:model/actions/countDraftRelations", get(content_manager_count_draft_relations))
        .route("/content-manager/collection-types/:uid/:id/actions/publish", post(publish_entry))
        .route("/content-manager/collection-types/:uid/:id/actions/unpublish", post(unpublish_entry))
        .route("/content-manager/collection-types/:uid/actions/bulkDelete", post(bulk_delete_entries))
        .route("/content-manager/collection-types/:uid/actions/bulkPublish", post(bulk_publish_entries))
        .route("/content-manager/collection-types/:uid/actions/bulkUnpublish", post(bulk_unpublish_entries))
        .route("/content-manager/single-types/:model", get(get_single_type).put(update_single_type))
        .route("/content-manager/preview/url/:content_type", get(content_manager_preview_url))
        .route("/content-manager/preview/script", get(get_preview_script))
        // --- Storefront Public High-Speed API ---
        .route("/api/:plural", get(public_get_collection))
        .layer(cors)
        .with_state(state);

    let port = 8088;
    let addr = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    info!("🚀 Zenith CMS Engine listening on http://{}", addr);
    info!("👉 Ready for instant hot-reload schema updates with 0-build & 0-restart!");

    axum::serve(listener, app).await?;
    Ok(())
}

// ========================
// ADMIN PROTOCOL HANDLERS
// ========================

async fn admin_init(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let admins = state.admins.read().await;
    let has_admin = !admins.is_empty();

    Json(json!({
        "data": {
            "uuid": false,
            "hasAdmin": has_admin,
            "menuLogo": "/logo-menu.png",
            "authLogo": "/logo.png"
        }
    }))
}

async fn admin_project_type() -> impl IntoResponse {
    Json(json!({
        "data": {
            "isEE": false,
            "isTrial": false,
            "features": [],
            "flags": {},
            "ai": { "enabled": false }
        }
    }))
}

async fn admin_project_settings() -> impl IntoResponse {
    Json(json!({
        "menuLogo": { "url": "/logo-menu.png" },
        "authLogo": { "url": "/logo.png" }
    }))
}

async fn admin_update_project_settings() -> impl IntoResponse {
    Json(json!({
        "menuLogo": { "url": "/logo-menu.png" },
        "authLogo": { "url": "/logo.png" }
    }))
}

async fn admin_information() -> impl IntoResponse {
    Json(json!({
        "data": {
            "currentEnvironment": "development",
            "autoReload": true, // MUST BE TRUE TO ENABLE CONTENT-TYPE BUILDER!
            "strapiVersion": "1.0.0 (AR-CMS Engine)",
            "dependencies": {
                "ar-cms-engine": "1.0.0",
                "ar-cms-runtime": "axum-sqlx",
                "database": "sqlite-turso-postgres"
            },
            "projectId": "ar-cms-001",
            "nodeVersion": "Rust Axum + SQLx",
            "communityEdition": true,
            "useYarn": false
        }
    }))
}

async fn admin_telemetry() -> impl IntoResponse {
    Json(json!({
        "data": {
            "useTypescriptOnServer": true,
            "useTypescriptOnAdmin": true
        }
    }))
}

async fn admin_plugins() -> impl IntoResponse {
    Json(json!({
        "plugins": [
            {
                "name": "content-manager",
                "displayName": "Content Manager",
                "description": "Strapi Content Manager",
                "packageName": "@strapi/content-manager"
            },
            {
                "name": "content-type-builder",
                "displayName": "Content-Type Builder",
                "description": "Strapi Content Type Builder",
                "packageName": "@strapi/content-type-builder"
            },
            {
                "name": "upload",
                "displayName": "Media Library",
                "description": "Media Library upload plugin",
                "packageName": "@strapi/upload"
            },
            {
                "name": "i18n",
                "displayName": "Internationalization",
                "description": "Multi-language i18n support",
                "packageName": "@strapi/i18n"
            }
        ]
    }))
}

async fn admin_license_limit_information() -> impl IntoResponse {
    Json(json!({
        "data": {
            "enforcementUserCount": false,
            "currentUsersCount": 1,
            "permittedSeats": 999999,
            "isTrial": false,
            "licenseLimitStatus": "ok"
        }
    }))
}

async fn admin_license_trial_time_left() -> impl IntoResponse {
    Json(json!({
        "data": {
            "trialTimeLeft": 0
        }
    }))
}

async fn admin_guided_tour_meta() -> impl IntoResponse {
    Json(json!({
        "data": {
            "guidedTourState": {}
        }
    }))
}

async fn admin_registration_info() -> impl IntoResponse {
    Json(json!({
        "data": {}
    }))
}

async fn admin_providers() -> impl IntoResponse {
    Json(json!([]))
}

async fn admin_providers_is_sso_locked() -> impl IntoResponse {
    Json(json!({
        "data": {
            "isSSOLocked": false
        }
    }))
}

async fn admin_providers_options() -> impl IntoResponse {
    Json(json!({
        "data": {
            "autoRegister": false,
            "defaultRole": null
        }
    }))
}

async fn admin_providers_options_update() -> impl IntoResponse {
    Json(json!({
        "data": {
            "autoRegister": false,
            "defaultRole": null
        }
    }))
}

async fn admin_permissions_check(
    Json(payload): Json<HashMap<String, Value>>,
) -> impl IntoResponse {
    let count = payload.get("permissions")
        .and_then(|p| p.as_array())
        .map(|arr| arr.len())
        .unwrap_or(1);

    let results: Vec<bool> = (0..count).map(|_| true).collect();
    Json(json!({ "data": results }))
}

async fn admin_debug_dump() -> impl IntoResponse {
    Json(json!({
        "data": {
            "app": {
                "strapiVersion": "5.63.0",
                "nodeVersion": "v20.0.0"
            }
        }
    }))
}

async fn admin_forgot_password() -> impl IntoResponse {
    Json(json!({ "data": "ok" }))
}

async fn admin_reset_password() -> impl IntoResponse {
    Json(json!({ "data": "ok" }))
}

async fn admin_access_token() -> impl IntoResponse {
    Json(json!({ "data": { "token": "zenith-jwt-token-admin" } }))
}

// ========================
// AUTHENTICATION HANDLERS
// ========================

#[derive(Deserialize)]
struct RegisterAdminPayload {
    firstname: String,
    lastname: Option<String>,
    email: String,
    password: Option<String>,
    #[serde(rename = "deviceId")]
    _device_id: Option<String>,
}

async fn admin_register_admin(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<RegisterAdminPayload>,
) -> impl IntoResponse {
    let mut admins = state.admins.write().await;
    let next_id = (admins.len() as u64) + 1;
    let doc_id = uuid::Uuid::new_v4().to_string();

    let user = AdminUser {
        id: next_id,
        document_id: doc_id,
        firstname: payload.firstname.clone(),
        lastname: payload.lastname.unwrap_or_default(),
        username: payload.email.clone(),
        email: payload.email.clone(),
        is_active: true,
        blocked: false,
        prefered_language: Some("en".to_string()),
        roles: vec![AdminRole {
            id: 1,
            name: "Super Admin".to_string(),
            code: "strapi-super-admin".to_string(),
            description: "Super Admin".to_string(),
            users_count: 1,
        }],
        password_hash: payload.password.unwrap_or_default(),
    };

    admins.push(user.clone());
    drop(admins);
    let _ = state.save_admins().await;
    info!("🎉 New administrator account created: {}", user.email);

    (StatusCode::CREATED, Json(json!({
        "data": {
            "token": "zenith-jwt-token-admin",
            "user": user
        }
    })))
}

async fn admin_register_user(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<RegisterAdminPayload>,
) -> impl IntoResponse {
    let mut admins = state.admins.write().await;
    let next_id = (admins.len() as u64) + 1;
    let doc_id = uuid::Uuid::new_v4().to_string();

    let user = AdminUser {
        id: next_id,
        document_id: doc_id,
        firstname: payload.firstname.clone(),
        lastname: payload.lastname.unwrap_or_default(),
        username: payload.email.clone(),
        email: payload.email.clone(),
        is_active: true,
        blocked: false,
        prefered_language: Some("en".to_string()),
        roles: vec![AdminRole {
            id: 2,
            name: "Editor".to_string(),
            code: "strapi-editor".to_string(),
            description: "Editor".to_string(),
            users_count: 1,
        }],
        password_hash: payload.password.unwrap_or_default(),
    };

    admins.push(user.clone());
    drop(admins);
    let _ = state.save_admins().await;

    (StatusCode::CREATED, Json(json!({
        "data": {
            "token": "zenith-jwt-token-user",
            "user": user
        }
    })))
}

async fn admin_login(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<HashMap<String, Value>>,
) -> impl IntoResponse {
    let admins = state.admins.read().await;
    if admins.is_empty() {
        return (StatusCode::BAD_REQUEST, Json(json!({
            "error": {
                "status": 400,
                "name": "ValidationError",
                "message": "No admin user registered yet. Please complete setup wizard."
            }
        }))).into_response();
    }

    let input_email = payload.get("email").and_then(|e| e.as_str()).unwrap_or("");
    let input_pass = payload.get("password").and_then(|p| p.as_str()).unwrap_or("");

    let matched = admins.iter().find(|u| u.email.eq_ignore_ascii_case(input_email));
    let user = match matched {
        Some(u) => {
            if !u.password_hash.is_empty() && !input_pass.is_empty() && u.password_hash != input_pass {
                return (StatusCode::BAD_REQUEST, Json(json!({
                    "error": {
                        "status": 400,
                        "name": "ValidationError",
                        "message": "Invalid email or password"
                    }
                }))).into_response();
            }
            u.clone()
        }
        None => admins.first().cloned().unwrap(),
    };

    (StatusCode::OK, Json(json!({
        "data": {
            "token": "zenith-jwt-token-admin",
            "user": user
        }
    }))).into_response()
}

async fn admin_renew_token() -> impl IntoResponse {
    Json(json!({
        "data": {
            "token": "zenith-jwt-token-admin"
        }
    }))
}

async fn admin_logout() -> impl IntoResponse {
    Json(json!({ "data": {} }))
}

async fn admin_users_me(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let admins = state.admins.read().await;
    if let Some(user) = admins.first() {
        (StatusCode::OK, Json(json!({ "data": user }))).into_response()
    } else {
        (StatusCode::UNAUTHORIZED, Json(json!({
            "data": null,
            "error": {
                "status": 401,
                "name": "UnauthorizedError",
                "message": "Missing or invalid credentials"
            }
        }))).into_response()
    }
}

async fn admin_users_me_update(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<HashMap<String, Value>>,
) -> impl IntoResponse {
    let mut admins = state.admins.write().await;
    if let Some(user) = admins.first_mut() {
        if let Some(fname) = payload.get("firstname").and_then(|v| v.as_str()) {
            user.firstname = fname.to_string();
        }
        if let Some(lname) = payload.get("lastname").and_then(|v| v.as_str()) {
            user.lastname = lname.to_string();
        }
        if let Some(email) = payload.get("email").and_then(|v| v.as_str()) {
            user.email = email.to_string();
            user.username = email.to_string();
        }
        let updated = user.clone();
        drop(admins);
        let _ = state.save_admins().await;
        return Json(json!({ "data": updated }));
    }

    Json(json!({
        "data": {
            "id": 1,
            "documentId": "zenith-admin-01",
            "firstname": "Admin",
            "lastname": "Superuser",
            "username": "admin",
            "email": "admin@pixelstack.io",
            "isActive": true,
            "blocked": false,
            "preferedLanguage": "en",
            "roles": [{ "id": 1, "name": "Super Admin", "code": "strapi-super-admin" }]
        }
    }))
}

async fn admin_users_me_permissions(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let actions = vec![
        "admin::roles.create",
        "admin::roles.update",
        "admin::roles.read",
        "admin::roles.delete",
        "admin::users.create",
        "admin::users.update",
        "admin::users.read",
        "admin::users.delete",
        "admin::project-settings.read",
        "admin::project-settings.update",
        "admin::webhooks.create",
        "admin::webhooks.update",
        "admin::webhooks.read",
        "admin::webhooks.delete",
        "admin::api-tokens.access",
        "admin::api-tokens.create",
        "admin::api-tokens.read",
        "admin::api-tokens.update",
        "admin::api-tokens.delete",
        "admin::api-tokens.regenerate",
        "admin::admin-tokens.access",
        "admin::admin-tokens.create",
        "admin::admin-tokens.read",
        "admin::admin-tokens.update",
        "admin::admin-tokens.delete",
        "admin::admin-tokens.regenerate",
        "admin::transfer.tokens.access",
        "admin::transfer.tokens.create",
        "admin::transfer.tokens.read",
        "admin::transfer.tokens.update",
        "admin::transfer.tokens.delete",
        "admin::transfer.tokens.regenerate",
        "admin::debug-dump.read",
        "plugin::content-type-builder.read",
        "plugin::content-type-builder.main",
        "plugin::content-manager.collection-types.configure-view",
        "plugin::content-manager.components.configure-layout",
        "plugin::content-manager.single-types.configure-view",
        "plugin::content-manager.explorer.read",
        "plugin::content-manager.explorer.create",
        "plugin::content-manager.explorer.update",
        "plugin::content-manager.explorer.delete",
        "plugin::content-manager.explorer.publish",
        "plugin::upload.read",
        "plugin::upload.assets.create",
        "plugin::upload.assets.update",
        "plugin::upload.assets.delete",
        "plugin::upload.assets.download",
        "plugin::upload.configure-view",
        "plugin::i18n.locale.read",
        "plugin::i18n.locale.create",
        "plugin::i18n.locale.update",
        "plugin::i18n.locale.delete",
    ];

    let mut permissions: Vec<Value> = actions
        .into_iter()
        .enumerate()
        .map(|(idx, act)| {
            json!({
                "id": (idx as u64) + 1,
                "action": act,
                "subject": null,
                "properties": {},
                "conditions": []
            })
        })
        .collect();

    // Populate explicit permissions for all registered collection types!
    let schemas = state.schemas.read().await;
    let mut id_counter = (permissions.len() as u64) + 1;
    let cm_actions = [
        "plugin::content-manager.explorer.read",
        "plugin::content-manager.explorer.create",
        "plugin::content-manager.explorer.update",
        "plugin::content-manager.explorer.delete",
        "plugin::content-manager.explorer.publish",
    ];

    for (uid, schema) in schemas.iter() {
        let mut field_names: Vec<String> = schema.attributes.keys().cloned().collect();
        field_names.sort();

        for action in &cm_actions {
            permissions.push(json!({
                "id": id_counter,
                "action": action,
                "subject": uid,
                "properties": {
                    "fields": field_names,
                    "locales": ["en"]
                },
                "conditions": []
            }));
            id_counter += 1;
        }
    }

    Json(json!({ "data": permissions }))
}

async fn admin_sessions_list() -> impl IntoResponse {
    Json(json!({ "data": [] }))
}

async fn admin_sessions_revoke_all() -> impl IntoResponse {
    Json(json!({ "data": {} }))
}

// ========================
// ROLES & USERS MANAGEMENT
// ========================

async fn admin_list_users(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let admins = state.admins.read().await;
    let list: Vec<AdminUser> = admins.clone();
    let total = list.len();
    Json(json!({
        "data": {
            "results": list,
            "pagination": { "page": 1, "pageSize": 25, "pageCount": 1, "total": total }
        }
    }))
}

async fn admin_create_user(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<HashMap<String, Value>>,
) -> impl IntoResponse {
    let mut admins = state.admins.write().await;
    let next_id = (admins.len() as u64) + 1;
    let fname = payload.get("firstname").and_then(|v| v.as_str()).unwrap_or("User");
    let lname = payload.get("lastname").and_then(|v| v.as_str()).unwrap_or("");
    let email = payload.get("email").and_then(|v| v.as_str()).unwrap_or("user@pixelstack.io");

    let new_user = AdminUser {
        id: next_id,
        document_id: uuid::Uuid::new_v4().to_string(),
        firstname: fname.to_string(),
        lastname: lname.to_string(),
        username: email.to_string(),
        email: email.to_string(),
        is_active: true,
        blocked: false,
        prefered_language: Some("en".to_string()),
        roles: vec![AdminRole {
            id: 2,
            name: "Editor".to_string(),
            code: "strapi-editor".to_string(),
            description: "Editor".to_string(),
            users_count: 1,
        }],
        password_hash: "".to_string(),
    };

    admins.push(new_user.clone());
    drop(admins);
    let _ = state.save_admins().await;

    (StatusCode::CREATED, Json(json!({ "data": new_user })))
}

async fn admin_get_user(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let admins = state.admins.read().await;
    if let Some(user) = admins.iter().find(|u| u.id.to_string() == id || u.document_id == id) {
        return (StatusCode::OK, Json(json!({ "data": user })));
    }
    (StatusCode::NOT_FOUND, Json(json!({ "error": "User not found" })))
}

async fn admin_update_user(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
    Json(payload): Json<HashMap<String, Value>>,
) -> impl IntoResponse {
    let mut admins = state.admins.write().await;
    if let Some(user) = admins.iter_mut().find(|u| u.id.to_string() == id || u.document_id == id) {
        if let Some(fname) = payload.get("firstname").and_then(|v| v.as_str()) {
            user.firstname = fname.to_string();
        }
        if let Some(lname) = payload.get("lastname").and_then(|v| v.as_str()) {
            user.lastname = lname.to_string();
        }
        let updated = user.clone();
        drop(admins);
        let _ = state.save_admins().await;
        return (StatusCode::OK, Json(json!({ "data": updated })));
    }
    (StatusCode::NOT_FOUND, Json(json!({ "error": "User not found" })))
}

async fn admin_delete_user(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let mut admins = state.admins.write().await;
    let before = admins.len();
    admins.retain(|u| u.id.to_string() != id && u.document_id != id);
    if admins.len() != before {
        drop(admins);
        let _ = state.save_admins().await;
        return (StatusCode::OK, Json(json!({ "data": { "id": id } })));
    }
    (StatusCode::NOT_FOUND, Json(json!({ "error": "User not found" })))
}

async fn admin_batch_delete_users() -> impl IntoResponse {
    Json(json!({ "data": {} }))
}

async fn admin_list_roles(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let roles = state.roles.read().await;
    Json(json!({ "data": *roles }))
}

async fn admin_create_role(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<HashMap<String, Value>>,
) -> impl IntoResponse {
    let mut roles = state.roles.write().await;
    let next_id = (roles.len() as u64) + 1;
    let name = payload.get("name").and_then(|v| v.as_str()).unwrap_or("Custom Role");
    let desc = payload.get("description").and_then(|v| v.as_str()).unwrap_or("");

    let role = AdminRole {
        id: next_id,
        name: name.to_string(),
        code: format!("custom-role-{}", next_id),
        description: desc.to_string(),
        users_count: 0,
    };

    roles.push(role.clone());
    (StatusCode::CREATED, Json(json!({ "data": role })))
}

async fn admin_get_role(
    Path(id): Path<u64>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let roles = state.roles.read().await;
    if let Some(role) = roles.iter().find(|r| r.id == id) {
        return (StatusCode::OK, Json(json!({ "data": role })));
    }
    (StatusCode::NOT_FOUND, Json(json!({ "error": "Role not found" })))
}

async fn admin_update_role(
    Path(id): Path<u64>,
    State(state): State<Arc<AppState>>,
    Json(payload): Json<HashMap<String, Value>>,
) -> impl IntoResponse {
    let mut roles = state.roles.write().await;
    if let Some(role) = roles.iter_mut().find(|r| r.id == id) {
        if let Some(name) = payload.get("name").and_then(|v| v.as_str()) {
            role.name = name.to_string();
        }
        if let Some(desc) = payload.get("description").and_then(|v| v.as_str()) {
            role.description = desc.to_string();
        }
        return (StatusCode::OK, Json(json!({ "data": role.clone() })));
    }
    (StatusCode::NOT_FOUND, Json(json!({ "error": "Role not found" })))
}

async fn admin_delete_role(
    Path(id): Path<u64>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let mut roles = state.roles.write().await;
    roles.retain(|r| r.id != id);
    (StatusCode::OK, Json(json!({ "data": { "id": id } })))
}

async fn admin_get_role_permissions(Path(_id): Path<u64>) -> impl IntoResponse {
    Json(json!({
        "data": [
            { "id": 1, "action": "plugin::content-manager.explorer.read", "subject": null, "properties": {}, "conditions": [] },
            { "id": 2, "action": "plugin::content-manager.explorer.create", "subject": null, "properties": {}, "conditions": [] },
            { "id": 3, "action": "plugin::content-manager.explorer.update", "subject": null, "properties": {}, "conditions": [] },
            { "id": 4, "action": "plugin::content-manager.explorer.delete", "subject": null, "properties": {}, "conditions": [] },
            { "id": 5, "action": "plugin::content-manager.explorer.publish", "subject": null, "properties": {}, "conditions": [] },
            { "id": 6, "action": "plugin::upload.read", "subject": null, "properties": {}, "conditions": [] },
            { "id": 7, "action": "plugin::upload.assets.create", "subject": null, "properties": {}, "conditions": [] }
        ]
    }))
}

async fn admin_update_role_permissions(Path(_id): Path<u64>) -> impl IntoResponse {
    Json(json!({ "data": [] }))
}

async fn admin_get_all_permissions(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let schemas = state.schemas.read().await;
    let subjects: Vec<Value> = schemas
        .iter()
        .map(|(uid, schema)| {
            json!({
                "uid": uid,
                "label": schema.info.display_name,
                "properties": [
                    { "label": "Fields", "value": "fields", "children": [] },
                    { "label": "Locales", "value": "locales", "children": [{ "label": "English (en)", "value": "en" }] }
                ]
            })
        })
        .collect();

    Json(json!({
        "data": {
            "conditions": [
                {
                    "id": "is-creator",
                    "displayName": "Is creator",
                    "category": "default"
                }
            ],
            "sections": {
                "collectionTypes": {
                    "actions": [
                        { "actionId": "plugin::content-manager.explorer.create", "applyToProperties": ["fields"], "label": "Create", "subjects": [] },
                        { "actionId": "plugin::content-manager.explorer.read", "applyToProperties": ["fields"], "label": "Read", "subjects": [] },
                        { "actionId": "plugin::content-manager.explorer.update", "applyToProperties": ["fields"], "label": "Update", "subjects": [] },
                        { "actionId": "plugin::content-manager.explorer.delete", "applyToProperties": [], "label": "Delete", "subjects": [] },
                        { "actionId": "plugin::content-manager.explorer.publish", "applyToProperties": [], "label": "Publish", "subjects": [] }
                    ],
                    "subjects": subjects
                },
                "singleTypes": {
                    "actions": [],
                    "subjects": []
                },
                "plugins": [
                    {
                        "action": "plugin::upload.read",
                        "displayName": "Access the Media Library",
                        "plugin": "upload",
                        "subCategory": "general"
                    },
                    {
                        "action": "plugin::upload.assets.create",
                        "displayName": "Create (upload)",
                        "plugin": "upload",
                        "subCategory": "assets"
                    },
                    {
                        "action": "plugin::upload.assets.update",
                        "displayName": "Update (crop, details)",
                        "plugin": "upload",
                        "subCategory": "assets"
                    },
                    {
                        "action": "plugin::upload.assets.delete",
                        "displayName": "Delete",
                        "plugin": "upload",
                        "subCategory": "assets"
                    }
                ],
                "settings": [
                    {
                        "action": "admin::users.read",
                        "displayName": "Read",
                        "category": "users and roles",
                        "subCategory": "users"
                    },
                    {
                        "action": "admin::users.create",
                        "displayName": "Create",
                        "category": "users and roles",
                        "subCategory": "users"
                    },
                    {
                        "action": "admin::roles.read",
                        "displayName": "Read",
                        "category": "users and roles",
                        "subCategory": "roles"
                    },
                    {
                        "action": "admin::roles.create",
                        "displayName": "Create",
                        "category": "users and roles",
                        "subCategory": "roles"
                    },
                    {
                        "action": "admin::webhooks.read",
                        "displayName": "Read",
                        "category": "webhooks",
                        "subCategory": "general"
                    },
                    {
                        "action": "admin::api-tokens.read",
                        "displayName": "Read",
                        "category": "api tokens",
                        "subCategory": "general"
                    },
                    {
                        "action": "admin::transfer.tokens.read",
                        "displayName": "Read",
                        "category": "transfer tokens",
                        "subCategory": "general"
                    }
                ]
            }
        }
    }))
}

// ========================
// WEBHOOKS MANAGEMENT
// ========================

async fn admin_list_webhooks(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let webhooks = state.webhooks.read().await;
    Json(json!({ "data": *webhooks }))
}

async fn admin_create_webhook(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<HashMap<String, Value>>,
) -> impl IntoResponse {
    let mut webhooks = state.webhooks.write().await;
    let id = uuid::Uuid::new_v4().to_string();
    let name = payload.get("name").and_then(|v| v.as_str()).unwrap_or("New Webhook");
    let url = payload.get("url").and_then(|v| v.as_str()).unwrap_or("https://example.com/webhook");

    let webhook = Webhook {
        id,
        name: name.to_string(),
        url: url.to_string(),
        headers: HashMap::new(),
        events: vec!["entry.create".to_string(), "entry.update".to_string()],
        is_enabled: true,
    };

    webhooks.push(webhook.clone());
    drop(webhooks);
    let _ = state.save_webhooks().await;

    (StatusCode::CREATED, Json(json!({ "data": webhook })))
}

async fn admin_get_webhook(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let webhooks = state.webhooks.read().await;
    if let Some(wh) = webhooks.iter().find(|w| w.id == id) {
        return (StatusCode::OK, Json(json!({ "data": wh })));
    }
    (StatusCode::NOT_FOUND, Json(json!({ "error": "Webhook not found" })))
}

async fn admin_update_webhook(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
    Json(payload): Json<HashMap<String, Value>>,
) -> impl IntoResponse {
    let mut webhooks = state.webhooks.write().await;
    if let Some(wh) = webhooks.iter_mut().find(|w| w.id == id) {
        if let Some(name) = payload.get("name").and_then(|v| v.as_str()) {
            wh.name = name.to_string();
        }
        if let Some(url) = payload.get("url").and_then(|v| v.as_str()) {
            wh.url = url.to_string();
        }
        let updated = wh.clone();
        drop(webhooks);
        let _ = state.save_webhooks().await;
        return (StatusCode::OK, Json(json!({ "data": updated })));
    }
    (StatusCode::NOT_FOUND, Json(json!({ "error": "Webhook not found" })))
}

async fn admin_delete_webhook(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let mut webhooks = state.webhooks.write().await;
    webhooks.retain(|w| w.id != id);
    drop(webhooks);
    let _ = state.save_webhooks().await;
    (StatusCode::OK, Json(json!({ "data": { "id": id } })))
}

async fn admin_batch_delete_webhooks() -> impl IntoResponse {
    Json(json!({ "data": {} }))
}

async fn admin_trigger_webhook(Path(_id): Path<String>) -> impl IntoResponse {
    Json(json!({
        "data": {
            "statusCode": 200,
            "message": "Webhook triggered successfully"
        }
    }))
}

// ========================
// API & ADMIN TOKENS
// ========================

async fn admin_list_api_tokens(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let tokens = state.api_tokens.read().await;
    Json(json!({ "data": *tokens }))
}

async fn admin_create_api_token(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<HashMap<String, Value>>,
) -> impl IntoResponse {
    let mut tokens = state.api_tokens.write().await;
    let next_id = (tokens.len() as u64) + 1;
    let now = chrono::Utc::now().to_rfc3339();
    let name = payload.get("name").and_then(|v| v.as_str()).unwrap_or("API Token");
    let desc = payload.get("description").and_then(|v| v.as_str()).unwrap_or("");
    let token_type = payload.get("type").and_then(|v| v.as_str()).unwrap_or("full-access");
    let access_key = format!("zenith_{}_{}", uuid::Uuid::new_v4().simple(), uuid::Uuid::new_v4().simple());

    let token = ApiToken {
        id: next_id,
        name: name.to_string(),
        description: desc.to_string(),
        token_type: token_type.to_string(),
        access_key,
        created_at: now.clone(),
        updated_at: now,
        last_used_at: None,
        lifespan: payload.get("lifespan").cloned(),
        expires_at: None,
        permissions: vec!["*".to_string()],
    };

    tokens.push(token.clone());
    drop(tokens);
    let _ = state.save_tokens().await;
    (StatusCode::CREATED, Json(json!({ "data": token })))
}

async fn admin_get_api_token(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let tokens = state.api_tokens.read().await;
    if let Some(t) = tokens.iter().find(|t| t.id.to_string() == id || t.name == id) {
        return (StatusCode::OK, Json(json!({ "data": t })));
    }
    (StatusCode::NOT_FOUND, Json(json!({ "error": "Token not found" })))
}

async fn admin_update_api_token(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
    Json(payload): Json<HashMap<String, Value>>,
) -> impl IntoResponse {
    let mut tokens = state.api_tokens.write().await;
    if let Some(t) = tokens.iter_mut().find(|t| t.id.to_string() == id || t.name == id) {
        if let Some(name) = payload.get("name").and_then(|v| v.as_str()) {
            t.name = name.to_string();
        }
        if let Some(desc) = payload.get("description").and_then(|v| v.as_str()) {
            t.description = desc.to_string();
        }
        t.updated_at = chrono::Utc::now().to_rfc3339();
        let clone = t.clone();
        drop(tokens);
        let _ = state.save_tokens().await;
        return (StatusCode::OK, Json(json!({ "data": clone })));
    }
    (StatusCode::NOT_FOUND, Json(json!({ "error": "Token not found" })))
}

async fn admin_delete_api_token(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let mut tokens = state.api_tokens.write().await;
    tokens.retain(|t| t.id.to_string() != id && t.name != id);
    drop(tokens);
    let _ = state.save_tokens().await;
    (StatusCode::OK, Json(json!({ "data": { "id": id } })))
}

async fn admin_regenerate_api_token(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let mut tokens = state.api_tokens.write().await;
    let token = if let Some(t) = tokens.iter_mut().find(|t| t.id.to_string() == id || t.name == id) {
        t.access_key = format!("zenith_{}_{}", uuid::Uuid::new_v4().simple(), uuid::Uuid::new_v4().simple());
        t.updated_at = chrono::Utc::now().to_rfc3339();
        t.clone()
    } else {
        let now = chrono::Utc::now().to_rfc3339();
        let num_id = id.parse::<u64>().unwrap_or((tokens.len() as u64) + 1);
        let new_t = ApiToken {
            id: num_id,
            name: format!("API Token {}", num_id),
            description: "".to_string(),
            token_type: "full-access".to_string(),
            access_key: format!("zenith_{}_{}", uuid::Uuid::new_v4().simple(), uuid::Uuid::new_v4().simple()),
            created_at: now.clone(),
            updated_at: now,
            last_used_at: None,
            lifespan: None,
            expires_at: None,
            permissions: vec!["*".to_string()],
        };
        tokens.push(new_t.clone());
        new_t
    };
    drop(tokens);
    let _ = state.save_tokens().await;
    (StatusCode::OK, Json(json!({
        "data": {
            "accessKey": token.access_key,
            "id": token.id,
            "name": token.name,
            "description": token.description,
            "type": token.token_type,
            "createdAt": token.created_at,
            "updatedAt": token.updated_at
        }
    })))
}

// Admin Tokens
async fn admin_list_admin_tokens(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let tokens = state.admin_tokens.read().await;
    Json(json!({ "data": *tokens }))
}

async fn admin_create_admin_token(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<HashMap<String, Value>>,
) -> impl IntoResponse {
    let mut tokens = state.admin_tokens.write().await;
    let next_id = (tokens.len() as u64) + 1;
    let now = chrono::Utc::now().to_rfc3339();
    let name = payload.get("name").and_then(|v| v.as_str()).unwrap_or("Admin Token");
    let desc = payload.get("description").and_then(|v| v.as_str()).unwrap_or("");
    let access_key = format!("adm_{}_{}", uuid::Uuid::new_v4().simple(), uuid::Uuid::new_v4().simple());

    let token = ApiToken {
        id: next_id,
        name: name.to_string(),
        description: desc.to_string(),
        token_type: "custom".to_string(),
        access_key,
        created_at: now.clone(),
        updated_at: now,
        last_used_at: None,
        lifespan: None,
        expires_at: None,
        permissions: vec!["*".to_string()],
    };

    tokens.push(token.clone());
    drop(tokens);
    let _ = state.save_tokens().await;
    (StatusCode::CREATED, Json(json!({ "data": token })))
}

async fn admin_get_admin_token(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let tokens = state.admin_tokens.read().await;
    if let Some(t) = tokens.iter().find(|t| t.id.to_string() == id || t.name == id) {
        return (StatusCode::OK, Json(json!({ "data": t })));
    }
    (StatusCode::NOT_FOUND, Json(json!({ "error": "Token not found" })))
}

async fn admin_update_admin_token(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
    Json(payload): Json<HashMap<String, Value>>,
) -> impl IntoResponse {
    let mut tokens = state.admin_tokens.write().await;
    if let Some(t) = tokens.iter_mut().find(|t| t.id.to_string() == id || t.name == id) {
        if let Some(name) = payload.get("name").and_then(|v| v.as_str()) {
            t.name = name.to_string();
        }
        if let Some(desc) = payload.get("description").and_then(|v| v.as_str()) {
            t.description = desc.to_string();
        }
        t.updated_at = chrono::Utc::now().to_rfc3339();
        let clone = t.clone();
        drop(tokens);
        let _ = state.save_tokens().await;
        return (StatusCode::OK, Json(json!({ "data": clone })));
    }
    (StatusCode::NOT_FOUND, Json(json!({ "error": "Token not found" })))
}

async fn admin_delete_admin_token(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let mut tokens = state.admin_tokens.write().await;
    tokens.retain(|t| t.id.to_string() != id && t.name != id);
    drop(tokens);
    let _ = state.save_tokens().await;
    (StatusCode::OK, Json(json!({ "data": { "id": id } })))
}

async fn admin_regenerate_admin_token(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let mut tokens = state.admin_tokens.write().await;
    let token = if let Some(t) = tokens.iter_mut().find(|t| t.id.to_string() == id || t.name == id) {
        t.access_key = format!("adm_{}_{}", uuid::Uuid::new_v4().simple(), uuid::Uuid::new_v4().simple());
        t.updated_at = chrono::Utc::now().to_rfc3339();
        t.clone()
    } else {
        let now = chrono::Utc::now().to_rfc3339();
        let num_id = id.parse::<u64>().unwrap_or((tokens.len() as u64) + 1);
        let new_t = ApiToken {
            id: num_id,
            name: format!("Admin Token {}", num_id),
            description: "".to_string(),
            token_type: "custom".to_string(),
            access_key: format!("adm_{}_{}", uuid::Uuid::new_v4().simple(), uuid::Uuid::new_v4().simple()),
            created_at: now.clone(),
            updated_at: now,
            last_used_at: None,
            lifespan: None,
            expires_at: None,
            permissions: vec!["*".to_string()],
        };
        tokens.push(new_t.clone());
        new_t
    };
    drop(tokens);
    let _ = state.save_tokens().await;
    (StatusCode::OK, Json(json!({
        "data": {
            "accessKey": token.access_key,
            "id": token.id,
            "name": token.name
        }
    })))
}

// Transfer Tokens
async fn admin_list_transfer_tokens(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let tokens = state.transfer_tokens.read().await;
    Json(json!({ "data": *tokens }))
}

async fn admin_create_transfer_token(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<HashMap<String, Value>>,
) -> impl IntoResponse {
    let mut tokens = state.transfer_tokens.write().await;
    let next_id = (tokens.len() as u64) + 1;
    let now = chrono::Utc::now().to_rfc3339();
    let name = payload.get("name").and_then(|v| v.as_str()).unwrap_or("Transfer Token");
    let access_key = format!("transfer_{}", uuid::Uuid::new_v4().simple());

    let token = ApiToken {
        id: next_id,
        name: name.to_string(),
        description: "".to_string(),
        token_type: "push-pull".to_string(),
        access_key,
        created_at: now.clone(),
        updated_at: now,
        last_used_at: None,
        lifespan: None,
        expires_at: None,
        permissions: vec!["push".to_string(), "pull".to_string()],
    };

    tokens.push(token.clone());
    drop(tokens);
    let _ = state.save_tokens().await;
    (StatusCode::CREATED, Json(json!({ "data": token })))
}

async fn admin_get_transfer_token(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let tokens = state.transfer_tokens.read().await;
    if let Some(t) = tokens.iter().find(|t| t.id.to_string() == id || t.name == id) {
        return (StatusCode::OK, Json(json!({ "data": t })));
    }
    (StatusCode::NOT_FOUND, Json(json!({ "error": "Token not found" })))
}

async fn admin_update_transfer_token(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
    Json(payload): Json<HashMap<String, Value>>,
) -> impl IntoResponse {
    let mut tokens = state.transfer_tokens.write().await;
    if let Some(t) = tokens.iter_mut().find(|t| t.id.to_string() == id || t.name == id) {
        if let Some(name) = payload.get("name").and_then(|v| v.as_str()) {
            t.name = name.to_string();
        }
        t.updated_at = chrono::Utc::now().to_rfc3339();
        let clone = t.clone();
        drop(tokens);
        let _ = state.save_tokens().await;
        return (StatusCode::OK, Json(json!({ "data": clone })));
    }
    (StatusCode::NOT_FOUND, Json(json!({ "error": "Token not found" })))
}

async fn admin_delete_transfer_token(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let mut tokens = state.transfer_tokens.write().await;
    tokens.retain(|t| t.id.to_string() != id && t.name != id);
    drop(tokens);
    let _ = state.save_tokens().await;
    (StatusCode::OK, Json(json!({ "data": { "id": id } })))
}

async fn admin_regenerate_transfer_token(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let mut tokens = state.transfer_tokens.write().await;
    let token = if let Some(t) = tokens.iter_mut().find(|t| t.id.to_string() == id || t.name == id) {
        t.access_key = format!("transfer_{}", uuid::Uuid::new_v4().simple());
        t.updated_at = chrono::Utc::now().to_rfc3339();
        t.clone()
    } else {
        let now = chrono::Utc::now().to_rfc3339();
        let num_id = id.parse::<u64>().unwrap_or((tokens.len() as u64) + 1);
        let new_t = ApiToken {
            id: num_id,
            name: format!("Transfer Token {}", num_id),
            description: "".to_string(),
            token_type: "push-pull".to_string(),
            access_key: format!("transfer_{}", uuid::Uuid::new_v4().simple()),
            created_at: now.clone(),
            updated_at: now,
            last_used_at: None,
            lifespan: None,
            expires_at: None,
            permissions: vec!["push".to_string(), "pull".to_string()],
        };
        tokens.push(new_t.clone());
        new_t
    };
    drop(tokens);
    let _ = state.save_tokens().await;
    (StatusCode::OK, Json(json!({
        "data": {
            "accessKey": token.access_key,
            "id": token.id,
            "name": token.name
        }
    })))
}

// ========================
// HOMEPAGE & WIDGETS
// ========================

async fn admin_homepage_key_statistics(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let admins_count = state.admins.read().await.len();
    let api_tokens_count = state.api_tokens.read().await.len();
    let webhooks_count = state.webhooks.read().await.len();
    let content_types_count = state.schemas.read().await.len();
    let entries_count: usize = state.entries.read().await.values().map(|e| e.len()).sum();

    Json(json!({
        "data": {
            "admins": admins_count,
            "apiTokens": api_tokens_count,
            "assets": 0,
            "components": 0,
            "contentTypes": content_types_count,
            "entries": entries_count,
            "locales": 1,
            "webhooks": webhooks_count
        }
    }))
}

async fn admin_homepage_recent_documents() -> impl IntoResponse {
    Json(json!({ "data": [] }))
}

async fn admin_homepage_count_documents() -> impl IntoResponse {
    Json(json!({
        "data": {
            "total": 3,
            "published": 3,
            "draft": 0
        }
    }))
}

async fn content_manager_count_documents(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let entries_map = state.entries.read().await;
    let mut published = 0;
    let mut draft = 0;

    for entries in entries_map.values() {
        for entry in entries {
            if entry.published_at.is_some() {
                published += 1;
            } else {
                draft += 1;
            }
        }
    }

    Json(json!({
        "data": {
            "draft": draft,
            "modified": 0,
            "published": published
        }
    }))
}

async fn admin_homepage_get_layout() -> impl IntoResponse {
    Json(json!({
        "data": {
            "layout": "default",
            "widgets": [],
            "updatedAt": "2026-11-18T00:00:00.000Z"
        }
    }))
}

async fn admin_homepage_update_layout() -> impl IntoResponse {
    Json(json!({
        "data": {
            "layout": "default",
            "widgets": [],
            "updatedAt": "2026-11-18T00:00:00.000Z"
        }
    }))
}

// ========================
// UPLOAD & MEDIA LIBRARY
// ========================

async fn upload_get_settings() -> impl IntoResponse {
    Json(json!({
        "data": {
            "autoOrientation": false,
            "sizeOptimization": true,
            "responsiveDimensions": true
        }
    }))
}

async fn upload_update_settings() -> impl IntoResponse {
    Json(json!({
        "data": {
            "autoOrientation": false,
            "sizeOptimization": true,
            "responsiveDimensions": true
        }
    }))
}

async fn upload_get_configuration() -> impl IntoResponse {
    Json(json!({
        "data": {
            "pageSize": 10,
            "sort": "createdAt:DESC"
        }
    }))
}

async fn upload_get_folders() -> impl IntoResponse {
    Json(json!({
        "data": []
    }))
}

async fn upload_list_files() -> impl IntoResponse {
    Json(json!({
        "results": [],
        "pagination": { "total": 0, "page": 1, "pageSize": 25, "pageCount": 1 }
    }))
}

async fn upload_file() -> impl IntoResponse {
    Json(json!({ "data": [] }))
}

// ========================
// i18n LOCALES
// ========================

async fn i18n_get_locales() -> impl IntoResponse {
    Json(json!([
        {
            "id": 1,
            "name": "English (en)",
            "code": "en",
            "isDefault": true,
            "createdAt": "2026-01-01T00:00:00.000Z",
            "updatedAt": "2026-01-01T00:00:00.000Z"
        }
    ]))
}

async fn i18n_create_locale(Json(payload): Json<Value>) -> impl IntoResponse {
    (StatusCode::CREATED, Json(json!({
        "id": 2,
        "name": payload.get("name").and_then(|v| v.as_str()).unwrap_or("Custom"),
        "code": payload.get("code").and_then(|v| v.as_str()).unwrap_or("custom"),
        "isDefault": false,
        "createdAt": chrono::Utc::now().to_rfc3339(),
        "updatedAt": chrono::Utc::now().to_rfc3339()
    })))
}

async fn i18n_get_iso_locales() -> impl IntoResponse {
    Json(json!([
        { "name": "English (en)", "code": "en" },
        { "name": "Persian (fa)", "code": "fa" },
        { "name": "French (fr)", "code": "fr" },
        { "name": "German (de)", "code": "de" },
        { "name": "Spanish (es)", "code": "es" }
    ]))
}

// ========================
// CONTENT-TYPE BUILDER HANDLERS
// ========================

fn format_schema_for_ctb(schema: &ContentTypeSchema) -> Value {
    let mut attrs_array = Vec::new();
    for (attr_name, attr_def) in &schema.attributes {
        let mut attr_obj = serde_json::to_value(attr_def).unwrap_or(json!({}));
        if let Some(obj) = attr_obj.as_object_mut() {
            obj.insert("name".to_string(), json!(attr_name));
        }
        attrs_array.push(attr_obj);
    }

    json!({
        "uid": schema.uid,
        "modelName": schema.info.singular_name,
        "apiID": schema.info.singular_name,
        "kind": schema.kind,
        "modelType": "contentType",
        "collectionName": schema.info.plural_name,
        "info": schema.info,
        "options": schema.options,
        "pluginOptions": {},
        "visible": true,
        "restrictRelationsTo": null,
        "attributes": attrs_array
    })
}

async fn get_all_schemas(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let schemas = state.schemas.read().await;
    let formatted_content_types: HashMap<String, Value> = schemas
        .iter()
        .map(|(uid, schema)| (uid.clone(), format_schema_for_ctb(schema)))
        .collect();

    Json(json!({
        "data": {
            "contentTypes": formatted_content_types,
            "components": {},
            "contentStructure": null,
            "settings": {
                "renameMigrations": {
                    "attributes": false
                }
            }
        }
    }))
}

async fn get_reserved_names() -> impl IntoResponse {
    let models = vec![
        "boolean", "date", "date_time", "time", "upload", "document", "then", "strapi*", "_strapi*", "__strapi*"
    ];
    let attributes = vec![
        "id", "document_id", "created_at", "updated_at", "published_at", "created_by_id", "updated_by_id", "created_by", "updated_by", "entry_id", "localizations", "meta", "locale", "__component", "__contentType", "strapi*", "_strapi*", "__strapi*", "status"
    ];
    Json(json!({
        "data": {
            "reservedNames": {
                "models": models,
                "attributes": attributes
            },
            "models": models,
            "attributes": attributes
        },
        "reservedNames": {
            "models": models,
            "attributes": attributes
        },
        "models": models,
        "attributes": attributes
    }))
}

#[derive(Deserialize)]
struct CreateContentTypePayload {
    #[serde(rename = "contentType")]
    content_type: ContentTypeSchema,
}

async fn create_or_update_content_type(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CreateContentTypePayload>,
) -> impl IntoResponse {
    let schema = payload.content_type;
    let uid = schema.uid.clone();

    if let Err(e) = state.persist_schema(&schema).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() })));
    }

    state.schemas.write().await.insert(uid.clone(), schema.clone());
    info!("⚡ Schema {} hot-reloaded into memory instantly!", schema.uid);

    (StatusCode::OK, Json(json!({ "data": format_schema_for_ctb(&schema) })))
}

async fn get_content_type_by_uid(
    Path(uid): Path<String>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let schemas = state.schemas.read().await;
    if let Some(schema) = schemas.get(&uid) {
        return (StatusCode::OK, Json(json!({ "data": format_schema_for_ctb(schema) })));
    }
    (StatusCode::NOT_FOUND, Json(json!({ "error": "Schema not found" })))
}

async fn update_content_type_by_uid(
    Path(uid): Path<String>,
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CreateContentTypePayload>,
) -> impl IntoResponse {
    let mut schema = payload.content_type;
    schema.uid = uid.clone();

    if let Err(e) = state.persist_schema(&schema).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() })));
    }

    state.schemas.write().await.insert(uid.clone(), schema.clone());
    info!("⚡ Schema {} hot-reloaded into memory instantly via PUT!", schema.uid);

    (StatusCode::OK, Json(json!({ "data": format_schema_for_ctb(&schema) })))
}

async fn delete_content_type(
    Path(uid): Path<String>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let mut schemas = state.schemas.write().await;
    if let Some(removed) = schemas.remove(&uid) {
        let file_name = format!("{}.json", removed.info.singular_name);
        let path = state.schemas_dir.join(file_name);
        let _ = fs::remove_file(path);
        (StatusCode::OK, Json(json!({ "data": { "uid": uid } })))
    } else {
        (StatusCode::NOT_FOUND, Json(json!({ "error": "Not found" })))
    }
}

async fn update_schema_batch(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<Value>,
) -> impl IntoResponse {
    info!("Batch schema update received: {:?}", payload);
    if let Some(content_types) = payload.get("data").and_then(|d| d.get("contentTypes")).and_then(|ct| ct.as_array()) {
        for ct in content_types {
            let action = ct.get("action").and_then(|a| a.as_str()).unwrap_or("");
            let uid = ct.get("uid").and_then(|u| u.as_str()).unwrap_or("");
            if uid.is_empty() {
                continue;
            }
            if action == "delete" {
                let mut schemas = state.schemas.write().await;
                if let Some(removed) = schemas.remove(uid) {
                    let file_name = format!("{}.json", removed.info.singular_name);
                    let _ = fs::remove_file(state.schemas_dir.join(file_name));
                    info!("🗑️ Deleted schema {} from disk", uid);
                }
            } else if action == "create" || action == "update" {
                let mut schemas = state.schemas.write().await;
                let mut schema = schemas.get(uid).cloned().unwrap_or_else(|| ContentTypeSchema {
                    uid: uid.to_string(),
                    kind: ct.get("kind").and_then(|k| k.as_str()).unwrap_or("collectionType").to_string(),
                    info: ContentTypeInfo {
                        singular_name: ct.get("singularName").and_then(|s| s.as_str()).unwrap_or("").to_string(),
                        plural_name: ct.get("pluralName").and_then(|p| p.as_str()).unwrap_or("").to_string(),
                        display_name: ct.get("displayName").and_then(|d| d.as_str()).unwrap_or("").to_string(),
                        description: ct.get("description").and_then(|d| d.as_str()).map(|s| s.to_string()),
                    },
                    options: ct.get("options").cloned(),
                    attributes: HashMap::new(),
                });

                if let Some(dn) = ct.get("displayName").and_then(|d| d.as_str()) {
                    schema.info.display_name = dn.to_string();
                }
                if let Some(sn) = ct.get("singularName").and_then(|s| s.as_str()) {
                    if !sn.is_empty() {
                        schema.info.singular_name = sn.to_string();
                    }
                }
                if let Some(pn) = ct.get("pluralName").and_then(|p| p.as_str()) {
                    if !pn.is_empty() {
                        schema.info.plural_name = pn.to_string();
                    }
                }
                if let Some(kind) = ct.get("kind").and_then(|k| k.as_str()) {
                    schema.kind = kind.to_string();
                }

                if let Some(attributes) = ct.get("attributes").and_then(|a| a.as_array()) {
                    for attr in attributes {
                        let attr_action = attr.get("action").and_then(|a| a.as_str()).unwrap_or("update");
                        let attr_name = attr.get("name").and_then(|n| n.as_str()).unwrap_or("");
                        if attr_name.is_empty() {
                            continue;
                        }
                        if attr_action == "delete" {
                            schema.attributes.remove(attr_name);
                        } else if let Some(props) = attr.get("properties") {
                            if let Ok(attr_def) = serde_json::from_value::<AttributeDefinition>(props.clone()) {
                                schema.attributes.insert(attr_name.to_string(), attr_def);
                            }
                        }
                    }
                }

                let _ = state.persist_schema(&schema).await;
                schemas.insert(uid.to_string(), schema);
                info!("⚡ Batch schema update successfully applied for {}", uid);
            }
        }
    }
    (StatusCode::OK, Json(json!({ "data": { "menu": {} } })))
}

async fn ctb_get_components() -> impl IntoResponse {
    Json(json!({ "data": [] }))
}

async fn ctb_get_component_by_uid(Path(uid): Path<String>) -> impl IntoResponse {
    Json(json!({ "data": { "uid": uid } }))
}

async fn get_update_schema_status() -> impl IntoResponse {
    Json(json!({
        "data": {
            "isUpdating": false
        }
    }))
}

// ========================
// CONTENT MANAGER HANDLERS
// ========================

async fn content_manager_init(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let schemas = state.schemas.read().await;
    let content_types: Vec<Value> = schemas.values().map(|s| {
        json!({
            "uid": s.uid,
            "isDisplayed": true,
            "apiID": s.info.singular_name,
            "kind": s.kind,
            "info": s.info,
            "options": s.options,
            "attributes": s.attributes,
            "pluginOptions": {
                "content-manager": { "visible": true },
                "content-type-builder": { "visible": true }
            }
        })
    }).collect();

    Json(json!({
        "data": {
            "fieldSizes": {},
            "components": [],
            "contentTypes": content_types
        }
    }))
}

async fn content_manager_uid_generate(Json(payload): Json<Value>) -> impl IntoResponse {
    let target = payload.get("data")
        .and_then(|d| d.get("target"))
        .and_then(|t| t.as_str())
        .unwrap_or("entry");
    let slug = target.to_lowercase().replace(' ', "-");
    Json(json!({ "data": slug }))
}

async fn content_manager_uid_check_availability() -> impl IntoResponse {
    Json(json!({ "isAvailable": true }))
}

async fn content_manager_get_components() -> impl IntoResponse {
    Json(json!({ "data": [] }))
}

async fn content_manager_get_component_configuration(Path(uid): Path<String>) -> impl IntoResponse {
    Json(json!({
        "data": {
            "component": {
                "uid": uid,
                "settings": {},
                "metadatas": {},
                "layouts": { "edit": [] }
            }
        }
    }))
}

async fn content_manager_update_component_configuration(Path(uid): Path<String>) -> impl IntoResponse {
    Json(json!({ "data": { "uid": uid } }))
}

async fn get_all_content_types(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let schemas = state.schemas.read().await;
    let list: Vec<ContentTypeSchema> = schemas.values().cloned().collect();
    Json(json!({ "data": list }))
}

fn get_schema_main_field(schema: &ContentTypeSchema) -> String {
    if schema.attributes.contains_key("title") {
        "title".to_string()
    } else if schema.attributes.contains_key("name") {
        "name".to_string()
    } else if schema.attributes.contains_key("label") {
        "label".to_string()
    } else if schema.attributes.contains_key("slug") {
        "slug".to_string()
    } else if let Some((key, _)) = schema.attributes.iter().find(|(_, attr)| attr.field_type == "string" || attr.field_type == "text" || attr.field_type == "email") {
        key.clone()
    } else {
        "id".to_string()
    }
}

async fn content_manager_count_draft_relations() -> impl IntoResponse {
    Json(json!({
        "data": {
            "unpublishedRelations": 0,
            "draftM2mLinks": 0
        }
    }))
}

async fn content_manager_preview_url() -> impl IntoResponse {
    Json(json!({
        "data": {
            "url": null
        }
    }))
}

async fn get_all_content_types_settings(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let schemas = state.schemas.read().await;
    let mut settings = Vec::new();

    for schema in schemas.values() {
        let main_field = get_schema_main_field(schema);
        settings.push(json!({
            "uid": schema.uid,
            "settings": {
                "bulkable": true,
                "filterable": true,
                "searchable": true,
                "pageSize": 25,
                "mainField": main_field,
                "defaultSortBy": "id",
                "defaultSortOrder": "ASC"
            }
        }));
    }

    Json(json!({ "data": settings }))
}

async fn get_content_type_configuration(
    Path(uid): Path<String>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let schemas = state.schemas.read().await;
    if let Some(schema) = schemas.get(&uid) {
        let main_field = get_schema_main_field(schema);
        let mut list_views = Vec::new();
        let mut edit_views = Vec::new();
        let mut metadatas = serde_json::Map::new();

        for key in schema.attributes.keys() {
            list_views.push(json!(key));
            edit_views.push(json!({ "name": key, "size": 6 }));
            metadatas.insert(key.clone(), json!({
                "edit": {
                    "label": key,
                    "description": "",
                    "placeholder": "",
                    "visible": true,
                    "editable": true
                },
                "list": {
                    "label": key,
                    "searchable": true,
                    "sortable": true
                }
            }));
        }

        return (StatusCode::OK, Json(json!({
            "data": {
                "contentType": {
                    "uid": uid,
                    "settings": {
                        "bulkable": true,
                        "filterable": true,
                        "searchable": true,
                        "pageSize": 25,
                        "mainField": main_field,
                        "defaultSortBy": "id",
                        "defaultSortOrder": "ASC"
                    },
                    "metadatas": metadatas,
                    "layouts": {
                        "list": list_views,
                        "edit": [edit_views]
                    }
                },
                "components": {}
            }
        })));
    }
    (StatusCode::NOT_FOUND, Json(json!({ "error": "Not found" })))
}

async fn update_content_type_configuration(
    Path(uid): Path<String>,
    Json(payload): Json<Value>,
) -> impl IntoResponse {
    Json(json!({ "data": { "uid": uid, "configuration": payload } }))
}

fn format_entry_data(entry: &ContentEntry) -> Value {
    let status = if entry.published_at.is_some() { "published" } else { "draft" };
    let mut map = serde_json::Map::new();
    for (k, v) in &entry.fields {
        if k != "id" && k != "documentId" && k != "createdAt" && k != "updatedAt" && k != "publishedAt" && k != "status" && k != "locale" {
            map.insert(k.clone(), v.clone());
        }
    }
    map.insert("id".to_string(), json!(entry.id));
    map.insert("documentId".to_string(), json!(entry.document_id));
    map.insert("createdAt".to_string(), json!(entry.created_at));
    map.insert("updatedAt".to_string(), json!(entry.updated_at));
    map.insert("publishedAt".to_string(), json!(entry.published_at));
    map.insert("locale".to_string(), json!(entry.locale.as_deref().unwrap_or("en")));
    map.insert("status".to_string(), json!(status));
    Value::Object(map)
}

fn format_entry_response(entry: &ContentEntry) -> Value {
    let data = format_entry_data(entry);
    let status = if entry.published_at.is_some() { "published" } else { "draft" };
    let locale = entry.locale.as_deref().unwrap_or("en");

    json!({
        "data": data,
        "meta": {
            "availableStatus": [
                {
                    "id": entry.id,
                    "documentId": entry.document_id,
                    "locale": locale,
                    "status": status,
                    "createdAt": entry.created_at,
                    "updatedAt": entry.updated_at,
                    "publishedAt": entry.published_at
                }
            ],
            "availableLocales": [],
            "defaultLocale": "en"
        }
    })
}

async fn get_entries(
    Path(uid): Path<String>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let entries_map = state.entries.read().await;
    let list = entries_map.get(&uid).cloned().unwrap_or_default();
    let formatted_list: Vec<Value> = list.iter().map(format_entry_data).collect();
    Json(json!({
        "results": formatted_list,
        "pagination": { "total": formatted_list.len(), "page": 1, "pageSize": 25, "pageCount": 1 }
    }))
}

async fn create_entry(
    Path(uid): Path<String>,
    State(state): State<Arc<AppState>>,
    Json(mut payload): Json<HashMap<String, Value>>,
) -> impl IntoResponse {
    let now = chrono::Utc::now().to_rfc3339();
    let doc_id = payload
        .remove("documentId")
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let locale = payload
        .remove("locale")
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap_or_else(|| "en".to_string());

    let mut entries_map = state.entries.write().await;
    let entries = entries_map.entry(uid.clone()).or_default();
    let next_id = (entries.len() as u64) + 1;

    let entry = ContentEntry {
        id: next_id,
        document_id: doc_id,
        created_at: now.clone(),
        updated_at: now,
        published_at: None,
        locale: Some(locale),
        status: Some("draft".to_string()),
        fields: payload,
    };

    entries.push(entry.clone());
    drop(entries_map);
    let _ = state.save_entries(&uid).await;
    (StatusCode::CREATED, Json(format_entry_response(&entry)))
}

async fn get_entry(
    Path((uid, id)): Path<(String, String)>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let mut entries_map = state.entries.write().await;
    let entries = entries_map.entry(uid.clone()).or_default();
    if let Some(entry) = entries.iter().find(|e| e.document_id == id || e.id.to_string() == id) {
        return (StatusCode::OK, Json(format_entry_response(entry)));
    }

    // Auto-heal / initialize draft entry if requested by UUID or string ID
    let now = chrono::Utc::now().to_rfc3339();
    let next_id = (entries.len() as u64) + 1;
    let new_entry = ContentEntry {
        id: next_id,
        document_id: id.clone(),
        created_at: now.clone(),
        updated_at: now,
        published_at: None,
        locale: Some("en".to_string()),
        status: Some("draft".to_string()),
        fields: HashMap::new(),
    };
    entries.push(new_entry.clone());
    drop(entries_map);
    let _ = state.save_entries(&uid).await;
    info!("Auto-initialized draft entry {} for {}", id, uid);

    (StatusCode::OK, Json(format_entry_response(&new_entry)))
}

async fn update_entry(
    Path((uid, id)): Path<(String, String)>,
    State(state): State<Arc<AppState>>,
    Json(mut payload): Json<HashMap<String, Value>>,
) -> impl IntoResponse {
    let mut entries_map = state.entries.write().await;
    let entries = entries_map.entry(uid.clone()).or_default();
    let entry = if let Some(existing) = entries.iter_mut().find(|e| e.document_id == id || e.id.to_string() == id) {
        existing.updated_at = chrono::Utc::now().to_rfc3339();
        for (k, v) in payload {
            if k != "id" && k != "documentId" && k != "createdAt" && k != "updatedAt" && k != "publishedAt" {
                existing.fields.insert(k, v);
            }
        }
        existing.clone()
    } else {
        let now = chrono::Utc::now().to_rfc3339();
        let next_id = (entries.len() as u64) + 1;
        payload.remove("id");
        payload.remove("documentId");
        payload.remove("createdAt");
        payload.remove("updatedAt");
        payload.remove("publishedAt");
        let new_entry = ContentEntry {
            id: next_id,
            document_id: id.clone(),
            created_at: now.clone(),
            updated_at: now,
            published_at: None,
            locale: Some("en".to_string()),
            status: Some("draft".to_string()),
            fields: payload,
        };
        entries.push(new_entry.clone());
        new_entry
    };
    drop(entries_map);
    let _ = state.save_entries(&uid).await;
    (StatusCode::OK, Json(format_entry_response(&entry)))
}

async fn delete_entry(
    Path((uid, id)): Path<(String, String)>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let mut entries_map = state.entries.write().await;
    if let Some(entries) = entries_map.get_mut(&uid) {
        let before_len = entries.len();
        entries.retain(|e| e.document_id != id && e.id.to_string() != id);
        if entries.len() != before_len {
            drop(entries_map);
            let _ = state.save_entries(&uid).await;
            return (StatusCode::OK, Json(json!({ "data": { "documentId": id } })));
        }
    }
    (StatusCode::NOT_FOUND, Json(json!({ "error": "Entry not found" })))
}

async fn publish_entry(
    Path((uid, id)): Path<(String, String)>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let mut entries_map = state.entries.write().await;
    if let Some(entries) = entries_map.get_mut(&uid) {
        if let Some(entry) = entries.iter_mut().find(|e| e.document_id == id || e.id.to_string() == id) {
            let now = chrono::Utc::now().to_rfc3339();
            entry.published_at = Some(now.clone());
            entry.updated_at = now;
            entry.status = Some("published".to_string());
            let clone = entry.clone();
            drop(entries_map);
            let _ = state.save_entries(&uid).await;
            return (StatusCode::OK, Json(format_entry_response(&clone)));
        }
    }
    (StatusCode::NOT_FOUND, Json(json!({ "error": "Entry not found" })))
}

async fn unpublish_entry(
    Path((uid, id)): Path<(String, String)>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let mut entries_map = state.entries.write().await;
    if let Some(entries) = entries_map.get_mut(&uid) {
        if let Some(entry) = entries.iter_mut().find(|e| e.document_id == id || e.id.to_string() == id) {
            let now = chrono::Utc::now().to_rfc3339();
            entry.published_at = None;
            entry.updated_at = now;
            entry.status = Some("draft".to_string());
            let clone = entry.clone();
            drop(entries_map);
            let _ = state.save_entries(&uid).await;
            return (StatusCode::OK, Json(format_entry_response(&clone)));
        }
    }
    (StatusCode::NOT_FOUND, Json(json!({ "error": "Entry not found" })))
}

#[derive(Deserialize, Debug, Default)]
struct BulkActionPayload {
    #[serde(default)]
    ids: Vec<Value>,
    #[serde(rename = "documentIds", default)]
    document_ids: Vec<String>,
}

fn extract_bulk_targets(payload: &BulkActionPayload) -> std::collections::HashSet<String> {
    let mut set = std::collections::HashSet::new();
    for doc_id in &payload.document_ids {
        set.insert(doc_id.clone());
    }
    for id_val in &payload.ids {
        if let Some(s) = id_val.as_str() {
            set.insert(s.to_string());
        } else if let Some(n) = id_val.as_u64() {
            set.insert(n.to_string());
        }
    }
    set
}

async fn bulk_delete_entries(
    Path(uid): Path<String>,
    State(state): State<Arc<AppState>>,
    Json(payload): Json<BulkActionPayload>,
) -> impl IntoResponse {
    let targets = extract_bulk_targets(&payload);
    let mut entries_map = state.entries.write().await;
    let mut count = 0;
    if let Some(entries) = entries_map.get_mut(&uid) {
        let before = entries.len();
        entries.retain(|e| !targets.contains(&e.document_id) && !targets.contains(&e.id.to_string()));
        count = before - entries.len();
    }
    drop(entries_map);
    let _ = state.save_entries(&uid).await;
    (StatusCode::OK, Json(json!({ "data": { "count": count } })))
}

async fn bulk_publish_entries(
    Path(uid): Path<String>,
    State(state): State<Arc<AppState>>,
    Json(payload): Json<BulkActionPayload>,
) -> impl IntoResponse {
    let targets = extract_bulk_targets(&payload);
    let mut entries_map = state.entries.write().await;
    let mut count = 0;
    if let Some(entries) = entries_map.get_mut(&uid) {
        for e in entries.iter_mut() {
            if targets.contains(&e.document_id) || targets.contains(&e.id.to_string()) {
                let now = chrono::Utc::now().to_rfc3339();
                e.published_at = Some(now.clone());
                e.updated_at = now;
                e.status = Some("published".to_string());
                count += 1;
            }
        }
    }
    drop(entries_map);
    let _ = state.save_entries(&uid).await;
    (StatusCode::OK, Json(json!({ "data": { "count": count } })))
}

async fn bulk_unpublish_entries(
    Path(uid): Path<String>,
    State(state): State<Arc<AppState>>,
    Json(payload): Json<BulkActionPayload>,
) -> impl IntoResponse {
    let targets = extract_bulk_targets(&payload);
    let mut entries_map = state.entries.write().await;
    let mut count = 0;
    if let Some(entries) = entries_map.get_mut(&uid) {
        for e in entries.iter_mut() {
            if targets.contains(&e.document_id) || targets.contains(&e.id.to_string()) {
                let now = chrono::Utc::now().to_rfc3339();
                e.published_at = None;
                e.updated_at = now;
                e.status = Some("draft".to_string());
                count += 1;
            }
        }
    }
    drop(entries_map);
    let _ = state.save_entries(&uid).await;
    (StatusCode::OK, Json(json!({ "data": { "count": count } })))
}

// Single Types
async fn get_single_type(
    Path(model): Path<String>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let singles = state.single_entries.read().await;
    let data = singles.get(&model).cloned().unwrap_or_default();
    Json(json!({ "data": data }))
}

async fn update_single_type(
    Path(model): Path<String>,
    State(state): State<Arc<AppState>>,
    Json(payload): Json<HashMap<String, Value>>,
) -> impl IntoResponse {
    let mut singles = state.single_entries.write().await;
    singles.insert(model, payload.clone());
    drop(singles);
    let _ = state.save_singles().await;
    (StatusCode::OK, Json(json!({ "data": payload })))
}

async fn get_preview_script() -> impl IntoResponse {
    (StatusCode::OK, [("content-type", "application/javascript")], "// Preview script loaded")
}

// ========================
// PUBLIC STOREFRONT HIGH-SPEED API
// ========================

#[derive(Deserialize)]
struct PublicQuery {
    #[serde(default)]
    _populate: Option<String>,
    #[serde(default)]
    _locale: Option<String>,
}

async fn public_get_collection(
    Path(plural): Path<String>,
    State(state): State<Arc<AppState>>,
    Query(_query): Query<PublicQuery>,
) -> impl IntoResponse {
    let schemas = state.schemas.read().await;
    let target_uid = schemas.values().find(|s| s.info.plural_name == plural).map(|s| s.uid.clone());

    if let Some(uid) = target_uid {
        let entries_map = state.entries.read().await;
        let published_entries: Vec<ContentEntry> = entries_map
            .get(&uid)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter(|e| e.published_at.is_some())
            .collect();

        return (
            StatusCode::OK,
            Json(json!({
                "data": published_entries,
                "meta": {
                    "pagination": {
                        "page": 1,
                        "pageSize": 25,
                        "pageCount": 1,
                        "total": published_entries.len()
                    }
                }
            })),
        );
    }

    (
        StatusCode::NOT_FOUND,
        Json(json!({
            "error": {
                "status": 404,
                "name": "NotFoundError",
                "message": format!("Endpoint /api/{} not found", plural)
            }
        })),
    )
}
