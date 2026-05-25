// SC-20..22 — authorization on PDM.

mod common;

use clientapi_pdm::apis::configuration::ApiKey;
use clientapi_pdm::apis::{access_acl_api, access_api, access_users_api, Error};
use clientapi_pdm::models::{
    AccessAclUpdateAclRequest, AccessUsersCreateTokenRequest, AccessUsersCreateUsersRequest,
};
use common::*;

const READONLY_USER: &str = "e2e-readonly@pdm";
const READONLY_TOKEN_ID: &str = "probe";
const ADMIN_PROBE_USER: &str = "e2e-admin-probe@pdm";

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sc_20_readonly_user_cannot_create() {
    let creds = Credentials::from_env();
    skip_if_pmg!(creds);

    let admin = creds.config_with_token();
    cleanup_e2e(&admin).await;

    let mut user_req = AccessUsersCreateUsersRequest::new(READONLY_USER.to_string());
    user_req.password = Some("dummy-password-not-used".to_string());
    access_users_api::access_users_create_users(&admin, user_req)
        .await
        .expect("create read-only user");

    // PDM ACL: `auth_id` (single string) + `role` (string, not enum).
    // Audit is the read-only role.
    access_acl_api::access_acl_update_acl(
        &admin,
        AccessAclUpdateAclRequest {
            path: "/".to_string(),
            role: "Auditor".to_string(),
            auth_id: Some(READONLY_USER.to_string()),
            propagate: Some(true),
            ..Default::default()
        },
    )
    .await
    .expect("grant Audit");

    let tok = access_users_api::access_users_create_token(
        &admin,
        READONLY_TOKEN_ID,
        READONLY_USER,
        Some(AccessUsersCreateTokenRequest::default()),
    )
    .await
    .expect("generate read-only token");

    let mut readonly_cfg = creds.config_anonymous();
    readonly_cfg.api_key = Some(ApiKey {
        prefix: None,
        key: format!(
            "PDMAPIToken={READONLY_USER}!{READONLY_TOKEN_ID}:{value}",
            value = tok.data.value
        ),
    });

    let err = access_users_api::access_users_create_users(
        &readonly_cfg,
        AccessUsersCreateUsersRequest::new("e2e-blocked@pdm".to_string()),
    )
    .await
    .expect_err("read-only must not create users");
    assert_response_status(&err, 403);

    cleanup_e2e(&admin).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sc_21_admin_can_create() {
    let creds = Credentials::from_env();
    skip_if_pmg!(creds);

    let admin = creds.config_with_token();
    let _ = access_users_api::access_users_delete_users(&admin, ADMIN_PROBE_USER, None).await;

    let mut req = AccessUsersCreateUsersRequest::new(ADMIN_PROBE_USER.to_string());
    req.password = Some("dummy-password".to_string());
    access_users_api::access_users_create_users(&admin, req)
        .await
        .expect("admin user creation");

    let _ = access_users_api::access_users_delete_users(&admin, ADMIN_PROBE_USER, None).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sc_22_permissions_returns_effective_acls() {
    let creds = Credentials::from_env();
    let admin = creds.config_with_token();

    let resp = access_api::access_get_permissions(&admin, Some(&creds.user), None)
        .await
        .expect("GET /access/permissions");

    let obj = resp
        .data
        .as_object()
        .expect("permissions data must be an object");
    assert!(!obj.is_empty(), "admin must have at least one ACL path");
}

fn assert_response_status<T: std::fmt::Debug>(err: &Error<T>, expected: u16) {
    match err {
        Error::ResponseError(rc) => assert_eq!(
            rc.status.as_u16(),
            expected,
            "expected HTTP {expected}, got {} (body: {})",
            rc.status,
            rc.content
        ),
        other => panic!("expected ResponseError({expected}), got {other:?}"),
    }
}
