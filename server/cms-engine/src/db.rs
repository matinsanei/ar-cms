use anyhow::Result;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use sqlx::Row;
use std::collections::HashMap;
use std::path::Path;
use std::str::FromStr;
use tracing::info;

use crate::models::{
    AdminRole, AdminUser, ApiToken, AuditLogEntry, ContentEntry, ContentRevision, ContentTypeSchema, MediaAsset, Webhook,
};

pub struct Database {
    pub pool: SqlitePool,
}

impl Database {
    pub async fn init(data_dir: &Path) -> Result<Self> {
        let db_path = data_dir.join("cms.db");
        let db_str = format!("sqlite://{}", db_path.to_string_lossy());

        info!("Initializing SQLite database at: {}", db_str);

        let opts = SqliteConnectOptions::from_str(&db_str)?
            .create_if_missing(true);

        let pool = SqlitePoolOptions::new()
            .max_connections(10)
            .connect_with(opts)
            .await?;

        // Initialize schema tables
        Self::create_tables(&pool).await?;

        Ok(Self { pool })
    }

    async fn create_tables(pool: &SqlitePool) -> Result<()> {
        sqlx::query(
            r#"
            PRAGMA journal_mode = WAL;
            PRAGMA foreign_keys = ON;

            CREATE TABLE IF NOT EXISTS cms_admins (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                document_id TEXT NOT NULL UNIQUE,
                email TEXT NOT NULL UNIQUE,
                username TEXT,
                firstname TEXT,
                lastname TEXT,
                password_hash TEXT NOT NULL,
                is_active INTEGER NOT NULL DEFAULT 1,
                blocked INTEGER NOT NULL DEFAULT 0,
                prefered_language TEXT DEFAULT 'en',
                roles_json TEXT NOT NULL DEFAULT '[]',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS cms_roles (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                code TEXT NOT NULL UNIQUE,
                description TEXT,
                permissions_json TEXT NOT NULL DEFAULT '[]'
            );

            CREATE TABLE IF NOT EXISTS cms_tokens (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                description TEXT,
                token_type TEXT NOT NULL,
                access_key TEXT NOT NULL UNIQUE,
                category TEXT NOT NULL,
                lifespan TEXT,
                expires_at TEXT,
                permissions_json TEXT NOT NULL DEFAULT '[]',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                last_used_at TEXT
            );

            CREATE TABLE IF NOT EXISTS cms_webhooks (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                url TEXT NOT NULL,
                headers_json TEXT NOT NULL DEFAULT '{}',
                events_json TEXT NOT NULL DEFAULT '[]',
                is_enabled INTEGER NOT NULL DEFAULT 1,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS cms_content_types (
                uid TEXT PRIMARY KEY,
                kind TEXT NOT NULL,
                singular_name TEXT NOT NULL,
                plural_name TEXT NOT NULL,
                display_name TEXT NOT NULL,
                schema_json TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS cms_entries (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                document_id TEXT NOT NULL,
                uid TEXT NOT NULL,
                locale TEXT DEFAULT 'en',
                status TEXT DEFAULT 'draft',
                published_at TEXT,
                fields_json TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS cms_media (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                document_id TEXT NOT NULL UNIQUE,
                name TEXT NOT NULL,
                alternative_text TEXT,
                caption TEXT,
                width INTEGER,
                height INTEGER,
                hash TEXT NOT NULL,
                ext TEXT NOT NULL,
                mime TEXT NOT NULL,
                size REAL NOT NULL,
                url TEXT NOT NULL,
                provider TEXT DEFAULT 'local',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS cms_audit_logs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id INTEGER,
                user_email TEXT,
                action TEXT NOT NULL,
                target_type TEXT,
                target_id TEXT,
                payload_json TEXT,
                ip TEXT,
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS cms_revisions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                uid TEXT NOT NULL,
                document_id TEXT NOT NULL,
                version INTEGER NOT NULL,
                status TEXT NOT NULL,
                author_email TEXT,
                snapshot_json TEXT NOT NULL,
                created_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_cms_revisions_doc ON cms_revisions(uid, document_id);
            "#,
        )
        .execute(pool)
        .await?;

        info!("Database tables verified and initialized successfully.");
        Ok(())
    }

    // ========================
    // ADMINS
    // ========================

    pub async fn load_admins(&self) -> Result<Vec<AdminUser>> {
        let rows = sqlx::query("SELECT id, document_id, email, username, firstname, lastname, password_hash, is_active, blocked, prefered_language, roles_json FROM cms_admins ORDER BY id ASC")
            .fetch_all(&self.pool)
            .await?;

        let mut admins = Vec::new();
        for r in rows {
            let roles_str: String = r.get("roles_json");
            let roles: Vec<AdminRole> = serde_json::from_str(&roles_str).unwrap_or_default();
            admins.push(AdminUser {
                id: r.get::<i64, _>("id") as u64,
                document_id: r.get("document_id"),
                firstname: r.get::<Option<String>, _>("firstname").unwrap_or_default(),
                lastname: r.get::<Option<String>, _>("lastname").unwrap_or_default(),
                username: r.get::<Option<String>, _>("username").unwrap_or_default(),
                email: r.get("email"),
                is_active: r.get::<i64, _>("is_active") != 0,
                blocked: r.get::<i64, _>("blocked") != 0,
                prefered_language: r.get("prefered_language"),
                roles,
                password_hash: r.get("password_hash"),
            });
        }
        Ok(admins)
    }

    pub async fn upsert_admin(&self, admin: &AdminUser) -> Result<()> {
        let roles_json = serde_json::to_string(&admin.roles)?;
        let now = chrono::Utc::now().to_rfc3339();

        sqlx::query(
            r#"
            INSERT INTO cms_admins (id, document_id, email, username, firstname, lastname, password_hash, is_active, blocked, prefered_language, roles_json, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?12)
            ON CONFLICT(email) DO UPDATE SET
                document_id = excluded.document_id,
                username = excluded.username,
                firstname = excluded.firstname,
                lastname = excluded.lastname,
                password_hash = CASE WHEN excluded.password_hash != '' THEN excluded.password_hash ELSE cms_admins.password_hash END,
                is_active = excluded.is_active,
                blocked = excluded.blocked,
                prefered_language = excluded.prefered_language,
                roles_json = excluded.roles_json,
                updated_at = excluded.updated_at
            "#
        )
        .bind(admin.id as i64)
        .bind(&admin.document_id)
        .bind(&admin.email)
        .bind(&admin.username)
        .bind(&admin.firstname)
        .bind(&admin.lastname)
        .bind(&admin.password_hash)
        .bind(if admin.is_active { 1i64 } else { 0i64 })
        .bind(if admin.blocked { 1i64 } else { 0i64 })
        .bind(&admin.prefered_language)
        .bind(roles_json)
        .bind(now)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn delete_admin(&self, id: u64) -> Result<()> {
        sqlx::query("DELETE FROM cms_admins WHERE id = ?1")
            .bind(id as i64)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // ========================
    // TOKENS
    // ========================

    pub async fn load_tokens(&self) -> Result<(Vec<ApiToken>, Vec<ApiToken>, Vec<ApiToken>)> {
        let rows = sqlx::query("SELECT id, name, description, token_type, access_key, category, lifespan, expires_at, permissions_json, created_at, updated_at, last_used_at FROM cms_tokens ORDER BY id ASC")
            .fetch_all(&self.pool)
            .await?;

        let mut api_tokens = Vec::new();
        let mut admin_tokens = Vec::new();
        let mut transfer_tokens = Vec::new();

        for r in rows {
            let category: String = r.get("category");
            let perms_str: String = r.get("permissions_json");
            let permissions: Vec<String> = serde_json::from_str(&perms_str).unwrap_or_default();
            let token = ApiToken {
                id: r.get::<i64, _>("id") as u64,
                name: r.get("name"),
                description: r.get("description"),
                token_type: r.get("token_type"),
                access_key: r.get("access_key"),
                created_at: r.get("created_at"),
                updated_at: r.get("updated_at"),
                last_used_at: r.get("last_used_at"),
                lifespan: r.get::<Option<String>, _>("lifespan").and_then(|s| serde_json::from_str(&s).ok()),
                expires_at: r.get("expires_at"),
                permissions,
            };

            match category.as_str() {
                "api" => api_tokens.push(token),
                "admin" => admin_tokens.push(token),
                "transfer" => transfer_tokens.push(token),
                _ => api_tokens.push(token),
            }
        }

        Ok((api_tokens, admin_tokens, transfer_tokens))
    }

    pub async fn upsert_token(&self, token: &ApiToken, category: &str) -> Result<()> {
        let perms_json = serde_json::to_string(&token.permissions)?;
        let lifespan_json = token.lifespan.as_ref().map(|v| v.to_string());

        sqlx::query(
            r#"
            INSERT INTO cms_tokens (id, name, description, token_type, access_key, category, lifespan, expires_at, permissions_json, created_at, updated_at, last_used_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
            ON CONFLICT(access_key) DO UPDATE SET
                name = excluded.name,
                description = excluded.description,
                token_type = excluded.token_type,
                lifespan = excluded.lifespan,
                expires_at = excluded.expires_at,
                permissions_json = excluded.permissions_json,
                updated_at = excluded.updated_at,
                last_used_at = excluded.last_used_at
            "#
        )
        .bind(token.id as i64)
        .bind(&token.name)
        .bind(&token.description)
        .bind(&token.token_type)
        .bind(&token.access_key)
        .bind(category)
        .bind(lifespan_json)
        .bind(&token.expires_at)
        .bind(perms_json)
        .bind(&token.created_at)
        .bind(&token.updated_at)
        .bind(&token.last_used_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn delete_token(&self, id: u64) -> Result<()> {
        sqlx::query("DELETE FROM cms_tokens WHERE id = ?1")
            .bind(id as i64)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // ========================
    // WEBHOOKS
    // ========================

    pub async fn load_webhooks(&self) -> Result<Vec<Webhook>> {
        let rows = sqlx::query("SELECT id, name, url, headers_json, events_json, is_enabled FROM cms_webhooks ORDER BY id ASC")
            .fetch_all(&self.pool)
            .await?;

        let mut webhooks = Vec::new();
        for r in rows {
            let headers_str: String = r.get("headers_json");
            let events_str: String = r.get("events_json");
            webhooks.push(Webhook {
                id: r.get("id"),
                name: r.get("name"),
                url: r.get("url"),
                headers: serde_json::from_str(&headers_str).unwrap_or_default(),
                events: serde_json::from_str(&events_str).unwrap_or_default(),
                is_enabled: r.get::<i64, _>("is_enabled") != 0,
            });
        }
        Ok(webhooks)
    }

    pub async fn upsert_webhook(&self, webhook: &Webhook) -> Result<()> {
        let headers_json = serde_json::to_string(&webhook.headers)?;
        let events_json = serde_json::to_string(&webhook.events)?;
        let now = chrono::Utc::now().to_rfc3339();

        sqlx::query(
            r#"
            INSERT INTO cms_webhooks (id, name, url, headers_json, events_json, is_enabled, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)
            ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                url = excluded.url,
                headers_json = excluded.headers_json,
                events_json = excluded.events_json,
                is_enabled = excluded.is_enabled,
                updated_at = excluded.updated_at
            "#
        )
        .bind(&webhook.id)
        .bind(&webhook.name)
        .bind(&webhook.url)
        .bind(headers_json)
        .bind(events_json)
        .bind(if webhook.is_enabled { 1i64 } else { 0i64 })
        .bind(now)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn delete_webhook(&self, id: &str) -> Result<()> {
        sqlx::query("DELETE FROM cms_webhooks WHERE id = ?1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // ========================
    // CONTENT TYPES & SCHEMAS
    // ========================

    pub async fn load_content_types(&self) -> Result<HashMap<String, ContentTypeSchema>> {
        let rows = sqlx::query("SELECT uid, schema_json FROM cms_content_types")
            .fetch_all(&self.pool)
            .await?;

        let mut map = HashMap::new();
        for r in rows {
            let uid: String = r.get("uid");
            let schema_str: String = r.get("schema_json");
            if let Ok(schema) = serde_json::from_str::<ContentTypeSchema>(&schema_str) {
                map.insert(uid, schema);
            }
        }
        Ok(map)
    }

    pub async fn upsert_content_type(&self, schema: &ContentTypeSchema) -> Result<()> {
        let schema_json = serde_json::to_string(schema)?;
        let now = chrono::Utc::now().to_rfc3339();

        sqlx::query(
            r#"
            INSERT INTO cms_content_types (uid, kind, singular_name, plural_name, display_name, schema_json, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)
            ON CONFLICT(uid) DO UPDATE SET
                kind = excluded.kind,
                singular_name = excluded.singular_name,
                plural_name = excluded.plural_name,
                display_name = excluded.display_name,
                schema_json = excluded.schema_json,
                updated_at = excluded.updated_at
            "#
        )
        .bind(&schema.uid)
        .bind(&schema.kind)
        .bind(&schema.info.singular_name)
        .bind(&schema.info.plural_name)
        .bind(&schema.info.display_name)
        .bind(schema_json)
        .bind(now)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn delete_content_type(&self, uid: &str) -> Result<()> {
        sqlx::query("DELETE FROM cms_content_types WHERE uid = ?1")
            .bind(uid)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // ========================
    // ENTRIES
    // ========================

    pub async fn load_all_entries(&self) -> Result<HashMap<String, Vec<ContentEntry>>> {
        let rows = sqlx::query("SELECT id, document_id, uid, locale, status, published_at, fields_json, created_at, updated_at FROM cms_entries ORDER BY id ASC")
            .fetch_all(&self.pool)
            .await?;

        let mut map: HashMap<String, Vec<ContentEntry>> = HashMap::new();
        for r in rows {
            let uid: String = r.get("uid");
            let fields_str: String = r.get("fields_json");
            let fields: HashMap<String, serde_json::Value> = serde_json::from_str(&fields_str).unwrap_or_default();
            let entry = ContentEntry {
                id: r.get::<i64, _>("id") as u64,
                document_id: r.get("document_id"),
                created_at: r.get("created_at"),
                updated_at: r.get("updated_at"),
                published_at: r.get("published_at"),
                locale: r.get("locale"),
                status: r.get("status"),
                fields,
            };
            map.entry(uid).or_default().push(entry);
        }
        Ok(map)
    }

    pub async fn upsert_entry(&self, uid: &str, entry: &ContentEntry) -> Result<()> {
        let fields_json = serde_json::to_string(&entry.fields)?;

        sqlx::query(
            r#"
            INSERT INTO cms_entries (id, document_id, uid, locale, status, published_at, fields_json, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            ON CONFLICT(id) DO UPDATE SET
                document_id = excluded.document_id,
                locale = excluded.locale,
                status = excluded.status,
                published_at = excluded.published_at,
                fields_json = excluded.fields_json,
                updated_at = excluded.updated_at
            "#
        )
        .bind(entry.id as i64)
        .bind(&entry.document_id)
        .bind(uid)
        .bind(&entry.locale)
        .bind(&entry.status)
        .bind(&entry.published_at)
        .bind(fields_json)
        .bind(&entry.created_at)
        .bind(&entry.updated_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn delete_entry(&self, uid: &str, id_or_doc_id: &str) -> Result<()> {
        sqlx::query("DELETE FROM cms_entries WHERE uid = ?1 AND (document_id = ?2 OR CAST(id AS TEXT) = ?2)")
            .bind(uid)
            .bind(id_or_doc_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // ========================
    // MEDIA ASSETS
    // ========================

    pub async fn load_media(&self) -> Result<Vec<MediaAsset>> {
        let rows = sqlx::query("SELECT id, document_id, name, alternative_text, caption, width, height, hash, ext, mime, size, url, provider, created_at, updated_at FROM cms_media ORDER BY id DESC")
            .fetch_all(&self.pool)
            .await?;

        let mut media_list = Vec::new();
        for r in rows {
            media_list.push(MediaAsset {
                id: r.get::<i64, _>("id") as u64,
                document_id: r.get("document_id"),
                name: r.get("name"),
                alternative_text: r.get("alternative_text"),
                caption: r.get("caption"),
                width: r.get::<Option<i64>, _>("width").map(|w| w as u32),
                height: r.get::<Option<i64>, _>("height").map(|h| h as u32),
                formats: None,
                hash: r.get("hash"),
                ext: r.get("ext"),
                mime: r.get("mime"),
                size: r.get::<f64, _>("size"),
                url: r.get("url"),
                preview_url: None,
                provider: r.get("provider"),
                provider_metadata: None,
                created_at: r.get("created_at"),
                updated_at: r.get("updated_at"),
            });
        }
        Ok(media_list)
    }

    pub async fn upsert_media(&self, media: &MediaAsset) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO cms_media (id, document_id, name, alternative_text, caption, width, height, hash, ext, mime, size, url, provider, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
            ON CONFLICT(document_id) DO UPDATE SET
                name = excluded.name,
                alternative_text = excluded.alternative_text,
                caption = excluded.caption,
                width = excluded.width,
                height = excluded.height,
                hash = excluded.hash,
                ext = excluded.ext,
                mime = excluded.mime,
                size = excluded.size,
                url = excluded.url,
                provider = excluded.provider,
                updated_at = excluded.updated_at
            "#
        )
        .bind(media.id as i64)
        .bind(&media.document_id)
        .bind(&media.name)
        .bind(&media.alternative_text)
        .bind(&media.caption)
        .bind(media.width.map(|w| w as i64))
        .bind(media.height.map(|h| h as i64))
        .bind(&media.hash)
        .bind(&media.ext)
        .bind(&media.mime)
        .bind(media.size)
        .bind(&media.url)
        .bind(&media.provider)
        .bind(&media.created_at)
        .bind(&media.updated_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn delete_media(&self, id: u64) -> Result<()> {
        sqlx::query("DELETE FROM cms_media WHERE id = ?1")
            .bind(id as i64)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // ========================
    // AUDIT LOGS
    // ========================

    pub async fn insert_audit_log(&self, entry: &AuditLogEntry) -> Result<()> {
        let payload_json = entry.payload.as_ref().map(|v| v.to_string());

        sqlx::query(
            r#"
            INSERT INTO cms_audit_logs (user_id, user_email, action, target_type, target_id, payload_json, ip, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            "#
        )
        .bind(entry.user_id.map(|u| u as i64))
        .bind(&entry.user_email)
        .bind(&entry.action)
        .bind(&entry.target_type)
        .bind(&entry.target_id)
        .bind(payload_json)
        .bind(&entry.ip)
        .bind(&entry.created_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn load_audit_logs(&self, limit: i64) -> Result<Vec<AuditLogEntry>> {
        let rows = sqlx::query("SELECT id, user_id, user_email, action, target_type, target_id, payload_json, ip, created_at FROM cms_audit_logs ORDER BY id DESC LIMIT ?1")
            .bind(limit)
            .fetch_all(&self.pool)
            .await?;

        let mut logs = Vec::new();
        for r in rows {
            let payload_str: Option<String> = r.get("payload_json");
            logs.push(AuditLogEntry {
                id: r.get::<i64, _>("id") as u64,
                user_id: r.get::<Option<i64>, _>("user_id").map(|u| u as u64),
                user_email: r.get("user_email"),
                action: r.get("action"),
                target_type: r.get("target_type"),
                target_id: r.get("target_id"),
                payload: payload_str.and_then(|s| serde_json::from_str(&s).ok()),
                ip: r.get("ip"),
                created_at: r.get("created_at"),
            });
        }
        Ok(logs)
    }

    // ========================
    // REVISIONS & VERSIONING
    // ========================

    pub async fn save_revision(&self, rev: &ContentRevision) -> Result<u64> {
        let snapshot_str = serde_json::to_string(&rev.snapshot)?;
        let res = sqlx::query(
            r#"
            INSERT INTO cms_revisions (uid, document_id, version, status, author_email, snapshot_json, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            "#,
        )
        .bind(&rev.uid)
        .bind(&rev.document_id)
        .bind(rev.version as i64)
        .bind(&rev.status)
        .bind(&rev.author_email)
        .bind(&snapshot_str)
        .bind(&rev.created_at)
        .execute(&self.pool)
        .await?;

        Ok(res.last_insert_rowid() as u64)
    }

    pub async fn load_revisions(&self, uid: &str, document_id: &str) -> Result<Vec<ContentRevision>> {
        let rows = sqlx::query(
            "SELECT id, uid, document_id, version, status, author_email, snapshot_json, created_at FROM cms_revisions WHERE uid = ?1 AND document_id = ?2 ORDER BY version DESC, id DESC"
        )
        .bind(uid)
        .bind(document_id)
        .fetch_all(&self.pool)
        .await?;

        let mut list = Vec::new();
        for r in rows {
            let snap_str: String = r.get("snapshot_json");
            let snap: serde_json::Value = serde_json::from_str(&snap_str).unwrap_or(serde_json::Value::Null);
            list.push(ContentRevision {
                id: r.get::<i64, _>("id") as u64,
                uid: r.get("uid"),
                document_id: r.get("document_id"),
                version: r.get::<i64, _>("version") as u32,
                status: r.get("status"),
                author_email: r.get("author_email"),
                snapshot: snap,
                created_at: r.get("created_at"),
            });
        }
        Ok(list)
    }

    pub async fn load_revision_by_id(&self, rev_id: u64) -> Result<Option<ContentRevision>> {
        let row = sqlx::query(
            "SELECT id, uid, document_id, version, status, author_email, snapshot_json, created_at FROM cms_revisions WHERE id = ?1"
        )
        .bind(rev_id as i64)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(r) = row {
            let snap_str: String = r.get("snapshot_json");
            let snap: serde_json::Value = serde_json::from_str(&snap_str).unwrap_or(serde_json::Value::Null);
            Ok(Some(ContentRevision {
                id: r.get::<i64, _>("id") as u64,
                uid: r.get("uid"),
                document_id: r.get("document_id"),
                version: r.get::<i64, _>("version") as u32,
                status: r.get("status"),
                author_email: r.get("author_email"),
                snapshot: snap,
                created_at: r.get("created_at"),
            }))
        } else {
            Ok(None)
        }
    }
}
