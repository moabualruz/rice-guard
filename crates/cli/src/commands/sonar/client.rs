/// SonarQube HTTP client with basic_auth token support.
use anyhow::{bail, Context};
use rguard_core::config::model::SonarqubeConfig;
use serde::de::DeserializeOwned;

use super::models::SonarResponse;

/// Async HTTP client for the SonarQube Web API.
///
/// Authenticates using HTTP Basic Auth with a user token (token as username,
/// empty password) as required by SonarQube's token auth scheme.
pub struct SonarClient {
    base_url: String,
    token: String,
    client: reqwest::Client,
}

impl SonarClient {
    /// Create a new `SonarClient`.
    ///
    /// `base_url` trailing slashes are trimmed. `token` is stored and injected
    /// per-request via `.basic_auth`.
    pub fn new(base_url: &str, token: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            token: token.to_string(),
            client: reqwest::Client::new(),
        }
    }

    /// Resolve the SonarQube token from environment or config.
    ///
    /// Priority: `SONARQUBE_TOKEN` env var > `config.token` field.
    /// Returns a descriptive error if neither is set.
    pub fn resolve_token(config: &SonarqubeConfig) -> anyhow::Result<String> {
        if let Ok(t) = std::env::var("SONARQUBE_TOKEN") {
            if !t.is_empty() {
                return Ok(t);
            }
        }
        if let Some(t) = &config.token {
            if !t.is_empty() {
                return Ok(t.clone());
            }
        }
        bail!(
            "SonarQube token not found. Set the SONARQUBE_TOKEN environment variable \
             or add `token: <your-token>` under `sonarqube:` in .rguard.yaml"
        )
    }

    /// Send a GET request to `path` and deserialize the JSON response as `T`.
    ///
    /// Returns a descriptive error on HTTP 401 or when the SonarQube response
    /// body contains an `errors` array (200-with-error pattern).
    pub async fn get<T: DeserializeOwned>(&self, path: &str) -> anyhow::Result<T> {
        let url = format!("{}{}", self.base_url, path);
        let resp = self
            .client
            .get(&url)
            .basic_auth(&self.token, None::<&str>)
            .send()
            .await
            .with_context(|| format!("GET {url}"))?;

        if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
            bail!("SonarQube auth failed — set SONARQUBE_TOKEN env var or check your token");
        }

        let status = resp.status();
        let body = resp
            .text()
            .await
            .with_context(|| format!("read body for GET {url}"))?;

        if !status.is_success() {
            bail!("SonarQube GET {url} returned HTTP {status}: {body}");
        }

        // Detect 200-with-errors pattern
        let wrapper: SonarResponse<T> = serde_json::from_str(&body)
            .with_context(|| format!("deserialize SonarQube response from GET {url}"))?;

        if let Some(errors) = wrapper.errors {
            if !errors.is_empty() {
                let msgs: Vec<_> = errors.iter().map(|e| e.msg.as_str()).collect();
                bail!("SonarQube error: {}", msgs.join("; "));
            }
        }

        wrapper
            .data
            .with_context(|| format!("SonarQube response for GET {url} contained no data"))
    }

    /// Send a POST with `application/x-www-form-urlencoded` body to `path`.
    ///
    /// Returns `Ok(())` on success, or a descriptive error on 401 / body errors.
    pub async fn post_form(&self, path: &str, params: &[(&str, &str)]) -> anyhow::Result<()> {
        let url = format!("{}{}", self.base_url, path);
        let resp = self
            .client
            .post(&url)
            .basic_auth(&self.token, None::<&str>)
            .form(params)
            .send()
            .await
            .with_context(|| format!("POST {url}"))?;

        if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
            bail!("SonarQube auth failed — set SONARQUBE_TOKEN env var or check your token");
        }

        let status = resp.status();
        let body = resp
            .text()
            .await
            .with_context(|| format!("read body for POST {url}"))?;

        if !status.is_success() {
            bail!("SonarQube POST {url} returned HTTP {status}: {body}");
        }

        // Check for 200-with-errors (errors array present and non-empty)
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&body) {
            if let Some(errors) = v.get("errors").and_then(|e| e.as_array()) {
                if !errors.is_empty() {
                    let msgs: Vec<_> = errors
                        .iter()
                        .filter_map(|e| e.get("msg").and_then(|m| m.as_str()))
                        .collect();
                    bail!("SonarQube error: {}", msgs.join("; "));
                }
            }
        }

        Ok(())
    }
}
