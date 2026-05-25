// SC-10..14 — authentication & CSRF on PDM.

mod common;

use clientapi_pdm::apis::configuration::ApiKey;
use clientapi_pdm::apis::{access_ticket_api, access_users_api, version_api, Error};
use clientapi_pdm::models::{AccessTicketCreateTicketRequest, AccessUsersCreateUsersRequest};
use common::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sc_10_ticket_auth_returns_ticket_and_csrf() {
    let creds = Credentials::from_env();

    // PDM ships the actual ticket in a Set-Cookie (PDMAuthCookie) header
    // rather than in the response body — the SDK strips response headers,
    // so we extract via raw reqwest (see common/credentials.rs).
    let (ticket, csrf) = pdm_raw_login(&creds).await.expect("PDM raw login");

    assert!(
        ticket.starts_with("PDM:"),
        "expected PDMAuthCookie to start with PDM:, got {ticket}"
    );
    assert!(!csrf.is_empty(), "csrf must be non-empty");

    let ticket_cfg = creds.config_with_ticket(&ticket, &csrf);
    version_api::version_get_version(&ticket_cfg)
        .await
        .expect("authenticated /version with ticket");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sc_11_ticket_auth_rejects_bad_password() {
    let creds = Credentials::from_env();
    let cfg = creds.config_anonymous();

    let mut req = AccessTicketCreateTicketRequest::new(creds.user.clone());
    req.password = Some("definitely-not-the-password".to_string());
    let err = access_ticket_api::access_ticket_create_ticket(&cfg, req)
        .await
        .expect_err("bad password must fail");
    assert_status(&err, 401);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sc_12_token_auth_returns_200() {
    let creds = Credentials::from_env();
    skip_if_pmg!(creds);

    let cfg = creds.config_with_token();
    version_api::version_get_version(&cfg)
        .await
        .expect("token auth /version");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sc_13_token_auth_rejects_malformed_token() {
    let creds = Credentials::from_env();
    skip_if_pmg!(creds);

    let mut cfg = creds.config_anonymous();
    cfg.api_key = Some(ApiKey {
        prefix: None,
        // PDM is a Rust-family product — `:` separator.
        key: "PDMAPIToken=root@pam!bogus:00000000-0000-0000-0000-000000000000".to_string(),
    });

    let err = version_api::version_get_version(&cfg)
        .await
        .expect_err("malformed token must fail");
    assert_status(&err, 401);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sc_14_state_change_without_csrf_returns_401() {
    let creds = Credentials::from_env();

    let (ticket, _csrf) = pdm_raw_login(&creds).await.expect("PDM raw login");
    let cfg = creds.config_with_ticket_no_csrf(&ticket);

    let _ = access_users_api::access_users_delete_users(
        &creds.config_with_token(),
        "e2e-csrf-probe@pdm",
        None,
    )
    .await;

    let req = AccessUsersCreateUsersRequest::new("e2e-csrf-probe@pdm".to_string());
    let err = access_users_api::access_users_create_users(&cfg, req)
        .await
        .expect_err("create_users without CSRF must fail");
    assert_status(&err, 401);
}

fn assert_status<T: std::fmt::Debug>(err: &Error<T>, expected: u16) {
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
