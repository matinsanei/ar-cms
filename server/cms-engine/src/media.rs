use anyhow::Result;
use axum::extract::Multipart;
use image::imageops::FilterType;
use image::{DynamicImage, GenericImageView, ImageFormat};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Cursor;
use std::path::Path;
use tracing::info;
use uuid::Uuid;

use crate::models::MediaAsset;

#[derive(Debug, Clone, Deserialize)]
pub struct TransformQuery {
    pub w: Option<u32>,
    pub width: Option<u32>,
    pub h: Option<u32>,
    pub height: Option<u32>,
    pub q: Option<u8>,
    pub quality: Option<u8>,
    pub format: Option<String>,
    pub fit: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageFormatMeta {
    pub name: String,
    pub hash: String,
    pub ext: String,
    pub mime: String,
    pub width: u32,
    pub height: u32,
    pub size: f64,
    pub url: String,
}

pub struct MediaService;

impl MediaService {
    pub async fn handle_upload(
        mut multipart: Multipart,
        uploads_dir: &Path,
        base_url: &str,
    ) -> Result<Vec<MediaAsset>> {
        let mut uploaded_assets = Vec::new();
        fs::create_dir_all(uploads_dir)?;

        let cache_dir = uploads_dir.join("cache");
        let _ = fs::create_dir_all(&cache_dir);

        while let Some(field) = multipart.next_field().await? {
            let file_name = field
                .file_name()
                .map(|s| s.to_string())
                .unwrap_or_else(|| format!("file_{}", Uuid::new_v4()));

            let bytes = field.bytes().await?;
            if bytes.is_empty() {
                continue;
            }

            // Compute SHA256
            let mut hasher = Sha256::new();
            hasher.update(&bytes);
            let hash = hex::encode(hasher.finalize());

            // Determine file extension
            let ext = std::path::Path::new(&file_name)
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| format!(".{}", e.to_lowercase()))
                .unwrap_or_else(|| ".bin".to_string());

            let mime = mime_guess::from_path(&file_name)
                .first_or_octet_stream()
                .to_string();

            let stored_filename = format!("{}{}", hash, ext);
            let target_path = uploads_dir.join(&stored_filename);

            fs::write(&target_path, &bytes)?;
            info!("Stored uploaded file: {:?}", target_path);

            let now = chrono::Utc::now().to_rfc3339();
            let size_kb = (bytes.len() as f64) / 1024.0;
            let doc_id = Uuid::new_v4().to_string();
            let asset_url = format!("{}/uploads/{}", base_url.trim_end_matches('/'), stored_filename);

            // Check if image and generate dimensions + responsive formats
            let (width, height, formats) = if mime.starts_with("image/") {
                if let Ok(img) = image::load_from_memory(&bytes) {
                    let (w, h) = img.dimensions();
                    let formats_obj = Self::generate_responsive_formats(&img, &hash, ext.as_str(), uploads_dir, base_url);
                    (Some(w), Some(h), formats_obj)
                } else {
                    (None, None, None)
                }
            } else {
                (None, None, None)
            };

            let asset = MediaAsset {
                id: (chrono::Utc::now().timestamp_millis() % 1_000_000) as u64,
                document_id: doc_id,
                name: file_name,
                alternative_text: None,
                caption: None,
                width,
                height,
                formats,
                hash,
                ext,
                mime,
                size: (size_kb * 100.0).round() / 100.0,
                url: asset_url,
                preview_url: None,
                provider: "local".to_string(),
                provider_metadata: None,
                created_at: now.clone(),
                updated_at: now,
            };

            uploaded_assets.push(asset);
        }

        Ok(uploaded_assets)
    }

    /// Automatically generate thumbnail, small, medium, large responsive formats
    fn generate_responsive_formats(
        img: &DynamicImage,
        hash: &str,
        ext: &str,
        uploads_dir: &Path,
        base_url: &str,
    ) -> Option<Value> {
        let (orig_w, orig_h) = img.dimensions();
        let mut formats_map = serde_json::Map::new();

        let targets = [
            ("thumbnail", 245, 156),
            ("small", 500, 500),
            ("medium", 750, 750),
            ("large", 1000, 1000),
        ];

        for (name, max_w, max_h) in targets {
            if orig_w > max_w || orig_h > max_h {
                let resized = img.resize(max_w, max_h, FilterType::Lanczos3);
                let (rw, rh) = resized.dimensions();
                let fmt_filename = format!("{}_{}{}", name, hash, ext);
                let fmt_path = uploads_dir.join(&fmt_filename);

                let mut buf = Cursor::new(Vec::new());
                let image_format = match ext {
                    ".png" => ImageFormat::Png,
                    ".webp" => ImageFormat::WebP,
                    _ => ImageFormat::Jpeg,
                };

                if resized.write_to(&mut buf, image_format).is_ok() {
                    let bytes = buf.into_inner();
                    let size_kb = (bytes.len() as f64) / 1024.0;
                    let _ = fs::write(fmt_path, bytes);

                    formats_map.insert(
                        name.to_string(),
                        json!({
                            "name": format!("{}_{}", name, hash),
                            "hash": format!("{}_{}", name, hash),
                            "ext": ext,
                            "mime": format!("image/{}", ext.trim_start_matches('.')),
                            "width": rw,
                            "height": rh,
                            "size": (size_kb * 100.0).round() / 100.0,
                            "url": format!("{}/uploads/{}", base_url.trim_end_matches('/'), fmt_filename)
                        }),
                    );
                }
            }
        }

        if formats_map.is_empty() {
            None
        } else {
            Some(Value::Object(formats_map))
        }
    }

    /// Dynamic On-the-fly Image Resizing & Format Optimization Worker
    pub fn process_dynamic_resize(
        uploads_dir: &Path,
        filename: &str,
        query: TransformQuery,
    ) -> Result<(Vec<u8>, &'static str)> {
        let source_path = uploads_dir.join(filename);
        if !source_path.exists() {
            anyhow::bail!("Source file not found");
        }

        let target_w = query.w.or(query.width).unwrap_or(0).clamp(0, 3840);
        let target_h = query.h.or(query.height).unwrap_or(0).clamp(0, 2160);
        let quality = query.q.or(query.quality).unwrap_or(80).clamp(1, 100);
        let fit = query.fit.unwrap_or_else(|| "contain".to_string());
        let target_format = query
            .format
            .as_deref()
            .unwrap_or("webp")
            .to_lowercase();

        // Check cache
        let cache_dir = uploads_dir.join("cache");
        let _ = fs::create_dir_all(&cache_dir);

        let cache_key = format!(
            "{}_w{}_h{}_q{}_{}.{}",
            filename, target_w, target_h, quality, fit, target_format
        );
        let cached_path = cache_dir.join(&cache_key);

        let mime_type = match target_format.as_str() {
            "png" => "image/png",
            "jpeg" | "jpg" => "image/jpeg",
            _ => "image/webp",
        };

        if cached_path.exists() {
            let cached_bytes = fs::read(&cached_path)?;
            return Ok((cached_bytes, mime_type));
        }

        // Process image on-the-fly
        let src_bytes = fs::read(&source_path)?;
        let img = image::load_from_memory(&src_bytes)?;

        let processed = if target_w > 0 && target_h > 0 {
            if fit == "cover" {
                img.resize_to_fill(target_w, target_h, FilterType::Lanczos3)
            } else {
                img.resize(target_w, target_h, FilterType::Lanczos3)
            }
        } else if target_w > 0 {
            img.resize(target_w, u32::MAX, FilterType::Lanczos3)
        } else if target_h > 0 {
            img.resize(u32::MAX, target_h, FilterType::Lanczos3)
        } else {
            img
        };

        let mut out_bytes = Cursor::new(Vec::new());
        let img_fmt = match target_format.as_str() {
            "png" => ImageFormat::Png,
            "jpeg" | "jpg" => ImageFormat::Jpeg,
            _ => ImageFormat::WebP,
        };

        processed.write_to(&mut out_bytes, img_fmt)?;
        let result = out_bytes.into_inner();

        // Write to cache for sub-millisecond future hits
        let _ = fs::write(cached_path, &result);

        Ok((result, mime_type))
    }
}
