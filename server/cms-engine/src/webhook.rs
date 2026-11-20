use anyhow::Result;
use reqwest::Client;
use serde_json::{json, Value};
use std::time::{Duration, Instant};
use tracing::{error, info};

use crate::models::Webhook;

#[derive(Clone)]
pub struct WebhookDispatcher {
    client: Client,
}

impl WebhookDispatcher {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap_or_default();
        Self { client }
    }

    /// Dispatch an event asynchronously to all matching enabled webhooks
    pub async fn dispatch(
        &self,
        webhooks: &[Webhook],
        event: &str,
        model: &str,
        entry: Value,
    ) {
        for wh in webhooks {
            if !wh.is_enabled {
                continue;
            }
            // Check if webhook is subscribed to this event
            let subscribed = wh.events.is_empty()
                || wh.events.iter().any(|e| e == event || e == "*" || e.starts_with(&format!("{}.", event)));
            if !subscribed {
                continue;
            }

            let client = self.client.clone();
            let url = wh.url.clone();
            let headers = wh.headers.clone();
            let event_name = event.to_string();
            let model_name = model.to_string();
            let payload = json!({
                "event": event_name,
                "createdAt": chrono::Utc::now().to_rfc3339(),
                "model": model_name,
                "entry": entry
            });

            tokio::spawn(async move {
                let mut req = client.post(&url).json(&payload);
                req = req.header("X-Strapi-Event", &event_name);
                for (k, v) in headers {
                    req = req.header(&k, &v);
                }

                match req.send().await {
                    Ok(resp) => {
                        info!("Webhook dispatched to {} - Status: {}", url, resp.status());
                    }
                    Err(e) => {
                        error!("Failed to dispatch webhook to {}: {:?}", url, e);
                    }
                }
            });
        }
    }

    /// Trigger a single webhook for testing purposes, returning status code and message
    pub async fn trigger_test(&self, webhook: &Webhook) -> Result<(u16, String, u64)> {
        let start = Instant::now();
        let payload = json!({
            "event": "trigger-test",
            "createdAt": chrono::Utc::now().to_rfc3339(),
            "data": {
                "message": "Hello from Matin CMS webhook trigger test!"
            }
        });

        let mut req = self.client.post(&webhook.url).json(&payload);
        req = req.header("X-Strapi-Event", "trigger-test");
        for (k, v) in &webhook.headers {
            req = req.header(k, v);
        }

        let resp = req.send().await?;
        let elapsed_ms = start.elapsed().as_millis() as u64;
        let status = resp.status().as_u16();
        let body_preview = resp.text().await.unwrap_or_else(|_| "OK".to_string());

        Ok((status, body_preview, elapsed_ms))
    }
}
