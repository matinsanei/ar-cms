use serde_json::{json, Value};
use std::collections::HashMap;

use crate::models::ContentEntry;

pub struct QueryResult {
    pub results: Vec<ContentEntry>,
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
    pub page_count: usize,
}

/// Calculate distance between two lat/lng points using Haversine formula (returns distance in meters)
pub fn haversine_distance(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let r = 6371000.0; // Earth radius in meters
    let phi1 = lat1.to_radians();
    let phi2 = lat2.to_radians();
    let delta_phi = (lat2 - lat1).to_radians();
    let delta_lambda = (lon2 - lon1).to_radians();

    let a = (delta_phi / 2.0).sin().powi(2)
        + phi1.cos() * phi2.cos() * (delta_lambda / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());

    r * c
}

/// Parse location coordinates from a field Value
fn extract_lat_lng(val: &Value) -> Option<(f64, f64)> {
    if let Some(obj) = val.as_object() {
        let lat = obj.get("lat")
            .or_else(|| obj.get("latitude"))
            .and_then(|v| v.as_f64());
        let lng = obj.get("lng")
            .or_else(|| obj.get("longitude"))
            .or_else(|| obj.get("lon"))
            .and_then(|v| v.as_f64());
        if let (Some(la), Some(lo)) = (lat, lng) {
            return Some((la, lo));
        }
    } else if let Some(arr) = val.as_array() {
        // GeoJSON [longitude, latitude] or [latitude, longitude]
        if arr.len() >= 2 {
            if let (Some(la), Some(lo)) = (arr[0].as_f64(), arr[1].as_f64()) {
                return Some((la, lo));
            }
        }
    }
    None
}

pub fn execute_query(
    entries: &[ContentEntry],
    params: &HashMap<String, String>,
) -> QueryResult {
    let mut filtered: Vec<ContentEntry> = entries.to_vec();

    // 1. Keyword search (_q)
    if let Some(q) = params.get("_q").or_else(|| params.get("q")) {
        let q_lower = q.to_lowercase();
        filtered.retain(|e| {
            if e.document_id.to_lowercase().contains(&q_lower) {
                return true;
            }
            for v in e.fields.values() {
                if let Some(s) = v.as_str() {
                    if s.to_lowercase().contains(&q_lower) {
                        return true;
                    }
                }
            }
            false
        });
    }

    // 2. Locale filter
    if let Some(loc) = params.get("locale").or_else(|| params.get("plugins[i18n][locale]")) {
        if loc != "all" && !loc.is_empty() {
            filtered.retain(|e| e.locale.as_deref().unwrap_or("en") == loc);
        }
    }

    // 3. Status / publicationState filter
    if let Some(status) = params.get("status") {
        if status == "draft" {
            filtered.retain(|e| e.published_at.is_none());
        } else if status == "published" {
            filtered.retain(|e| e.published_at.is_some());
        }
    } else if let Some(pub_state) = params.get("publicationState") {
        if pub_state == "live" {
            filtered.retain(|e| e.published_at.is_some());
        }
    }

    // 4. Geo-Spatial filtering ($near, $within)
    // E.g. filters[location][$near]=35.6892,51.3890,5000 (lat, lng, max_meters)
    for (key, val) in params {
        if key.contains("[$near]") {
            let field = key
                .strip_prefix("filters[")
                .and_then(|s| s.split(']').next())
                .unwrap_or("location");

            let parts: Vec<&str> = val.split(',').collect();
            if parts.len() >= 2 {
                let target_lat: f64 = parts[0].trim().parse().unwrap_or(0.0);
                let target_lng: f64 = parts[1].trim().parse().unwrap_or(0.0);
                let max_dist: f64 = parts.get(2).and_then(|d| d.trim().parse().ok()).unwrap_or(10_000.0); // 10km default

                filtered.retain(|e| {
                    if let Some(field_val) = e.fields.get(field) {
                        if let Some((e_lat, e_lng)) = extract_lat_lng(field_val) {
                            let dist = haversine_distance(target_lat, target_lng, e_lat, e_lng);
                            return dist <= max_dist;
                        }
                    }
                    false
                });
            }
        }
    }

    // 5. Standard field filters: e.g. filters[title][$contains]=foo or filters[id][$eq]=1
    for (key, val) in params {
        if key.starts_with("filters[") && !key.contains("[$near]") {
            let inner = key.strip_prefix("filters[").unwrap_or("");
            if let Some(field_end) = inner.find(']') {
                let field = &inner[..field_end];
                let op = if let Some(op_start) = inner.find("[$") {
                    if let Some(op_end) = inner[op_start..].find(']') {
                        &inner[op_start + 2..op_start + op_end]
                    } else {
                        "eq"
                    }
                } else {
                    "eq"
                };

                let val_lower = val.to_lowercase();
                filtered.retain(|e| {
                    let field_val = match field {
                        "id" => Some(e.id.to_string()),
                        "documentId" => Some(e.document_id.clone()),
                        "createdAt" => Some(e.created_at.clone()),
                        "updatedAt" => Some(e.updated_at.clone()),
                        "publishedAt" => e.published_at.clone(),
                        _ => e.fields.get(field).and_then(|v| {
                            if let Some(s) = v.as_str() {
                                Some(s.to_string())
                            } else if let Some(n) = v.as_i64() {
                                Some(n.to_string())
                            } else if let Some(f) = v.as_f64() {
                                Some(f.to_string())
                            } else if let Some(b) = v.as_bool() {
                                Some(b.to_string())
                            } else {
                                None
                            }
                        }),
                    };

                    match (op, field_val) {
                        ("eq", Some(actual)) => actual.eq_ignore_ascii_case(val),
                        ("eq", None) => false,
                        ("ne", Some(actual)) => !actual.eq_ignore_ascii_case(val),
                        ("ne", None) => true,
                        ("contains", Some(actual)) => actual.to_lowercase().contains(&val_lower),
                        ("contains", None) => false,
                        ("startsWith", Some(actual)) => actual.to_lowercase().starts_with(&val_lower),
                        ("startsWith", None) => false,
                        ("endsWith", Some(actual)) => actual.to_lowercase().ends_with(&val_lower),
                        ("endsWith", None) => false,
                        ("null", actual) => (val == "true" && actual.is_none()) || (val == "false" && actual.is_some()),
                        _ => false,
                    }
                });
            }
        }
    }

    // 6. Sorting (including distance sorting: sort=location:asc&origin=lat,lng)
    let sort_param = params.get("sort").cloned().unwrap_or_else(|| "id:ASC".to_string());
    let mut sort_parts = sort_param.split(':');
    let sort_field = sort_parts.next().unwrap_or("id");
    let sort_dir = sort_parts.next().unwrap_or("ASC").to_uppercase();
    let is_desc = sort_dir == "DESC";

    if let Some(origin_str) = params.get("origin") {
        let oparts: Vec<&str> = origin_str.split(',').collect();
        if oparts.len() >= 2 {
            let o_lat: f64 = oparts[0].trim().parse().unwrap_or(0.0);
            let o_lng: f64 = oparts[1].trim().parse().unwrap_or(0.0);

            filtered.sort_by(|a, b| {
                let dist_a = a.fields.get(sort_field)
                    .and_then(extract_lat_lng)
                    .map(|(la, lo)| haversine_distance(o_lat, o_lng, la, lo))
                    .unwrap_or(f64::MAX);

                let dist_b = b.fields.get(sort_field)
                    .and_then(extract_lat_lng)
                    .map(|(la, lo)| haversine_distance(o_lat, o_lng, la, lo))
                    .unwrap_or(f64::MAX);

                dist_a.partial_cmp(&dist_b).unwrap_or(std::cmp::Ordering::Equal)
            });
        }
    } else {
        filtered.sort_by(|a, b| {
            let cmp = match sort_field {
                "id" => a.id.cmp(&b.id),
                "createdAt" => a.created_at.cmp(&b.created_at),
                "updatedAt" => a.updated_at.cmp(&b.updated_at),
                "publishedAt" => a.published_at.cmp(&b.published_at),
                custom => {
                    let get_num = |e: &ContentEntry| {
                        e.fields.get(custom).and_then(|v| {
                            v.as_f64().or_else(|| v.as_i64().map(|n| n as f64)).or_else(|| {
                                v.as_str().and_then(|s| s.parse::<f64>().ok())
                            })
                        })
                    };
                    if let (Some(num_a), Some(num_b)) = (get_num(a), get_num(b)) {
                        num_a.partial_cmp(&num_b).unwrap_or(std::cmp::Ordering::Equal)
                    } else {
                        let val_a = a.fields.get(custom).and_then(|v| v.as_str()).unwrap_or("");
                        let val_b = b.fields.get(custom).and_then(|v| v.as_str()).unwrap_or("");
                        val_a.cmp(val_b)
                    }
                }
            };
            if is_desc {
                cmp.reverse()
            } else {
                cmp
            }
        });
    }

    // 7. Pagination
    let total = filtered.len();
    let page: usize = params
        .get("pagination[page]")
        .or_else(|| params.get("page"))
        .and_then(|p| p.parse().ok())
        .unwrap_or(1)
        .max(1);

    let page_size: usize = params
        .get("pagination[pageSize]")
        .or_else(|| params.get("pageSize"))
        .and_then(|ps| ps.parse().ok())
        .unwrap_or(25)
        .clamp(1, 100);

    let page_count = if total == 0 { 1 } else { (total + page_size - 1) / page_size };
    let start_idx = (page - 1) * page_size;
    let results = if start_idx < total {
        filtered.into_iter().skip(start_idx).take(page_size).collect()
    } else {
        Vec::new()
    };

    QueryResult {
        results,
        total,
        page,
        page_size,
        page_count,
    }
}

/// Convert collection entries into GeoJSON FeatureCollection
pub fn to_geojson(entries: &[ContentEntry], geo_field: &str) -> Value {
    let mut features = Vec::new();

    for entry in entries {
        if let Some(val) = entry.fields.get(geo_field) {
            if let Some((lat, lng)) = extract_lat_lng(val) {
                let mut props = serde_json::Map::new();
                for (k, v) in &entry.fields {
                    if k != geo_field {
                        props.insert(k.clone(), v.clone());
                    }
                }
                props.insert("id".to_string(), json!(entry.id));
                props.insert("documentId".to_string(), json!(entry.document_id));

                features.push(json!({
                    "type": "Feature",
                    "geometry": {
                        "type": "Point",
                        "coordinates": [lng, lat]
                    },
                    "properties": props
                }));
            }
        }
    }

    json!({
        "type": "FeatureCollection",
        "features": features
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn mock_entry(id: u64, doc_id: &str, title: &str, price: i64, lat: Option<f64>, lng: Option<f64>) -> ContentEntry {
        let mut fields = HashMap::new();
        fields.insert("title".to_string(), json!(title));
        fields.insert("price".to_string(), json!(price));
        if let (Some(la), Some(lo)) = (lat, lng) {
            fields.insert("location".to_string(), json!({ "lat": la, "lng": lo }));
        }

        ContentEntry {
            id,
            document_id: doc_id.to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
            published_at: Some("2026-01-01T00:00:00Z".to_string()),
            locale: Some("en".to_string()),
            status: Some("published".to_string()),
            fields,
        }
    }

    #[test]
    fn test_haversine_distance() {
        // Tehran (35.6892, 51.3890) to Isfahan (32.6546, 51.6680) ~ 340 km
        let dist = haversine_distance(35.6892, 51.3890, 32.6546, 51.6680);
        assert!(dist > 330_000.0 && dist < 350_000.0, "Tehran to Isfahan should be approx 340km");

        // Same point distance must be 0
        let zero_dist = haversine_distance(35.6892, 51.3890, 35.6892, 51.3890);
        assert!(zero_dist < 1.0, "Distance to self must be zero");
    }

    #[test]
    fn test_filter_exact_and_substring() {
        let entries = vec![
            mock_entry(1, "d1", "Apple iPhone 15", 999, None, None),
            mock_entry(2, "d2", "Samsung Galaxy S24", 899, None, None),
            mock_entry(3, "d3", "Google Pixel 8", 699, None, None),
        ];

        // $eq filter
        let mut params = HashMap::new();
        params.insert("filters[title][$eq]".to_string(), "Google Pixel 8".to_string());
        let res = execute_query(&entries, &params);
        assert_eq!(res.results.len(), 1);
        assert_eq!(res.results[0].id, 3);

        // $contains filter
        let mut params2 = HashMap::new();
        params2.insert("filters[title][$contains]".to_string(), "galaxy".to_string());
        let res2 = execute_query(&entries, &params2);
        assert_eq!(res2.results.len(), 1);
        assert_eq!(res2.results[0].id, 2);
    }

    #[test]
    fn test_spatial_proximity_query() {
        let entries = vec![
            mock_entry(1, "d1", "Tehran Grand Bazaar", 100, Some(35.6749), Some(51.4215)),
            mock_entry(2, "d2", "Tajrish Square", 200, Some(35.8052), Some(51.4300)),
            mock_entry(3, "d3", "Isfahan Naqsh-e Jahan", 300, Some(32.6577), Some(51.6775)),
        ];

        // Query within 15km of Tehran Grand Bazaar
        let mut params = HashMap::new();
        params.insert("filters[location][$near]".to_string(), "35.6749,51.4215,16000".to_string());
        let res = execute_query(&entries, &params);

        // Both Grand Bazaar and Tajrish (~14km) should match; Isfahan (~340km) should be excluded
        assert_eq!(res.results.len(), 2);
        let ids: Vec<u64> = res.results.iter().map(|e| e.id).collect();
        assert!(ids.contains(&1));
        assert!(ids.contains(&2));
        assert!(!ids.contains(&3));
    }

    #[test]
    fn test_to_geojson_conversion() {
        let entries = vec![
            mock_entry(1, "doc-1", "Point Alpha", 50, Some(35.7000), Some(51.4000)),
            mock_entry(2, "doc-2", "Point Beta", 80, None, None), // No location
        ];

        let geojson = to_geojson(&entries, "location");
        assert_eq!(geojson["type"], "FeatureCollection");
        let features = geojson["features"].as_array().unwrap();
        assert_eq!(features.len(), 1); // Only entry with valid location included

        let feat = &features[0];
        assert_eq!(feat["geometry"]["type"], "Point");
        assert_eq!(feat["geometry"]["coordinates"][0], 51.4000); // Lng
        assert_eq!(feat["geometry"]["coordinates"][1], 35.7000); // Lat
        assert_eq!(feat["properties"]["title"], "Point Alpha");
    }

    #[test]
    fn test_pagination_and_sorting() {
        let entries = vec![
            mock_entry(1, "d1", "Item A", 30, None, None),
            mock_entry(2, "d2", "Item B", 10, None, None),
            mock_entry(3, "d3", "Item C", 20, None, None),
            mock_entry(4, "d4", "Item D", 40, None, None),
        ];

        // Sort by id DESC, page 1, pageSize 2
        let mut params = HashMap::new();
        params.insert("sort".to_string(), "id:DESC".to_string());
        params.insert("pagination[page]".to_string(), "1".to_string());
        params.insert("pagination[pageSize]".to_string(), "2".to_string());

        let res = execute_query(&entries, &params);
        assert_eq!(res.total, 4);
        assert_eq!(res.page_count, 2);
        assert_eq!(res.results.len(), 2);
        assert_eq!(res.results[0].id, 4);
        assert_eq!(res.results[1].id, 3);
    }
}
