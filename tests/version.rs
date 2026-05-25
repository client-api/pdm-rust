// SC-01 — /version on PDM.

mod common;

use clientapi_pdm::apis::version_api;
use common::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sc_01_version_returns_expected_shape() {
    let creds = Credentials::from_env();
    let cfg = creds.config_with_token();

    let resp = version_api::version_get_version(&cfg)
        .await
        .expect("GET /version");

    // PDM 1.x shares the `version`/`release` shape with PBS. Don't pin
    // an exact major — accept anything non-empty in both fields.
    assert!(
        !resp.data.version.is_empty(),
        "version must be non-empty, got {:?}",
        resp.data.version
    );
    assert!(
        !resp.data.release.is_empty(),
        "release must be non-empty"
    );
}
