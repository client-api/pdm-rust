//! Example: list cluster nodes.
//!
//! Run with:
//!
//! ```sh
//! PDM_HOST=https://pdm.example.com:8443 \
//! PDM_TOKEN='root@pam!auto:...' \
//! cargo run --example list_nodes
//! ```

use clientapi_pdm::apis::configuration::Configuration;
use clientapi_pdm::apis::nodes_api;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut cfg = Configuration::new();
    cfg.base_path = format!(
        "{}/api2/json",
        std::env::var("PDM_HOST").unwrap_or_else(|_| "https://localhost:8443".into()),
    );
    cfg.bearer_access_token = std::env::var("PDM_TOKEN").ok();

    let resp = nodes_api::nodes_get_nodes(&cfg).await?;
    // Non-PVE products: the upstream apidoc.js declares this endpoint
    // `returns: { type: null }`, so the generator emits `data` as an
    // untyped `serde_json::Value` (or similar). Print whatever came back.
    println!("Response: {:?}", resp);
    Ok(())
}
