// SC-41 — error envelope handling on PDM.
// SC-40 (unknown vmid) and SC-42 (privsep) don't apply: no qemu/lxc on
// PDM, and PDM's create_token has no `privsep` parameter.

mod common;

use clientapi_pdm::apis::{access_users_api, Error};
use clientapi_pdm::models::AccessUsersCreateUsersRequest;
use common::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sc_41_invalid_input_returns_4xx() {
    let creds = Credentials::from_env();
    let cfg = creds.config_with_token();

    let req = AccessUsersCreateUsersRequest::new(String::new());
    let err = access_users_api::access_users_create_users(&cfg, req)
        .await
        .expect_err("empty userid must fail");

    match err {
        Error::ResponseError(rc) => {
            assert!(
                rc.status.is_client_error(),
                "expected 4xx, got {} (body: {})",
                rc.status,
                rc.content
            );
        }
        other => panic!("expected ResponseError, got {other:?}"),
    }
}
