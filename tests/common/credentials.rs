use clientapi_pdm::apis::configuration::{ApiKey, Configuration};
use reqwest::header::{HeaderMap, HeaderValue};

/// PDM is a Rust-family product — `PDMAPIToken=user@realm!id:uuid` with
/// `:` separator. `token_header_value` is pre-assembled by the container.
#[derive(Debug, Clone)]
pub struct Credentials {
    pub url: String,
    pub user: String,
    pub password: String,
    pub token_header_value: String,
    pub token_value: String,
}

impl Credentials {
    pub fn from_env() -> Self {
        Self {
            url: env_required("PROXMOX_URL"),
            user: env_required("PROXMOX_USER"),
            password: env_required("PROXMOX_PASSWORD"),
            token_header_value: env_required("PROXMOX_TOKEN_HEADER_VALUE"),
            token_value: env_required("PROXMOX_TOKEN_VALUE"),
        }
    }

    pub fn token_auth_supported(&self) -> bool {
        self.token_value != "(unsupported-by-pmg)"
    }

    fn base_client() -> reqwest::Client {
        reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .build()
            .expect("reqwest client")
    }

    pub fn config_anonymous(&self) -> Configuration {
        let mut cfg = Configuration::new();
        cfg.base_path = self.url.trim_end_matches('/').to_string() + "/api2/json";
        cfg.client = Self::base_client();
        cfg.api_key = None;
        cfg
    }

    pub fn config_with_token(&self) -> Configuration {
        let mut cfg = self.config_anonymous();
        cfg.api_key = Some(ApiKey {
            prefix: None,
            key: self.token_header_value.clone(),
        });
        cfg
    }

    pub fn config_with_ticket(&self, ticket: &str, csrf: &str) -> Configuration {
        let mut headers = HeaderMap::new();
        headers.insert(
            reqwest::header::COOKIE,
            HeaderValue::from_str(&format!("__Host-PDMAuthCookie={ticket}"))
                .expect("ticket header value"),
        );
        headers.insert(
            "CSRFPreventionToken",
            HeaderValue::from_str(csrf).expect("csrf header value"),
        );
        let client = reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .default_headers(headers)
            .build()
            .expect("reqwest client");
        let mut cfg = self.config_anonymous();
        cfg.client = client;
        cfg
    }

    pub fn config_with_ticket_no_csrf(&self, ticket: &str) -> Configuration {
        let mut headers = HeaderMap::new();
        headers.insert(
            reqwest::header::COOKIE,
            HeaderValue::from_str(&format!("__Host-PDMAuthCookie={ticket}"))
                .expect("ticket header value"),
        );
        let client = reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .default_headers(headers)
            .build()
            .expect("reqwest client");
        let mut cfg = self.config_anonymous();
        cfg.client = client;
        cfg
    }
}

fn env_required(key: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| panic!("env var {key} must be set"))
}

/// PDM-specific raw login that returns (auth-cookie-value, csrf-token).
///
/// Unlike PVE/PBS/PMG, PDM's `POST /access/ticket` response body does NOT
/// include the `ticket` field — the ticket is delivered as an HttpOnly
/// `__Host-PDMAuthCookie` Set-Cookie header. The SDK ignores response headers, so
/// the test code calls the endpoint via raw reqwest and extracts the
/// cookie itself.
pub async fn pdm_raw_login(creds: &Credentials) -> anyhow::Result<(String, String)> {
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()?;
    let url = format!(
        "{}/api2/json/access/ticket",
        creds.url.trim_end_matches('/')
    );
    let resp = client
        .post(&url)
        .form(&[("username", &creds.user), ("password", &creds.password)])
        .send()
        .await?
        .error_for_status()?;

    let cookie_value = resp
        .headers()
        .get_all(reqwest::header::SET_COOKIE)
        .iter()
        .find_map(|v| {
            let s = v.to_str().ok()?;
            let head = s.split(';').next()?;
            let (k, val) = head.split_once('=')?;
            (k == "__Host-PDMAuthCookie").then(|| val.to_string())
        })
        .ok_or_else(|| anyhow::anyhow!("__Host-PDMAuthCookie not present in Set-Cookie"))?;

    let body: serde_json::Value = resp.json().await?;
    let csrf = body
        .get("data")
        .and_then(|d| d.get("CSRFPreventionToken"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("CSRFPreventionToken missing in response body"))?
        .to_string();

    Ok((cookie_value, csrf))
}
