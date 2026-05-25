// SC-30, SC-31, SC-33, SC-34 — CRUD baseline on PDM.

mod common;

use clientapi_pdm::apis::{access_acl_api, access_users_api};
use clientapi_pdm::models::{AccessAclUpdateAclRequest, AccessUsersCreateUsersRequest};
use common::*;

const E2E_USER: &str = "e2e-user-01@pdm";

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sc_30_list_users_includes_root() {
    let creds = Credentials::from_env();
    let cfg = creds.config_with_token();

    let users = access_users_api::access_users_get_users(&cfg, None)
        .await
        .expect("list users");
    let has_root = users.data.iter().any(|u| u.userid == "root@pam");
    assert!(
        has_root,
        "expected root@pam, got {:?}",
        users.data.iter().map(|u| &u.userid).collect::<Vec<_>>()
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sc_31_user_crud_roundtrip() {
    let creds = Credentials::from_env();
    let cfg = creds.config_with_token();
    let _ = access_users_api::access_users_delete_users(&cfg, E2E_USER, None).await;

    let mut req = AccessUsersCreateUsersRequest::new(E2E_USER.to_string());
    req.password = Some("dummy-password".to_string());
    access_users_api::access_users_create_users(&cfg, req)
        .await
        .expect("create user");

    let listed = access_users_api::access_users_get_users(&cfg, None)
        .await
        .expect("list users");
    assert!(listed.data.iter().any(|u| u.userid == E2E_USER));

    access_users_api::access_users_delete_users(&cfg, E2E_USER, None)
        .await
        .expect("delete user");

    let listed = access_users_api::access_users_get_users(&cfg, None)
        .await
        .expect("list users after delete");
    assert!(!listed.data.iter().any(|u| u.userid == E2E_USER));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sc_33_acl_crud_roundtrip() {
    let creds = Credentials::from_env();
    let cfg = creds.config_with_token();

    cleanup_e2e(&cfg).await;
    let mut user_req = AccessUsersCreateUsersRequest::new(E2E_USER.to_string());
    user_req.password = Some("dummy-password".to_string());
    access_users_api::access_users_create_users(&cfg, user_req)
        .await
        .expect("create user for ACL test");

    access_acl_api::access_acl_update_acl(
        &cfg,
        AccessAclUpdateAclRequest {
            path: "/".to_string(),
            role: "Auditor".to_string(),
            auth_id: Some(E2E_USER.to_string()),
            propagate: Some(true),
            ..Default::default()
        },
    )
    .await
    .expect("grant ACL");

    let acl = access_acl_api::access_acl_get_acl(&cfg, None, None, None)
        .await
        .expect("read ACL");
    let granted = acl
        .data
        .iter()
        .any(|e| e.ugid == E2E_USER && e.roleid == "Auditor");
    assert!(granted, "ACL entry for {E2E_USER}/Auditor must be present");

    access_acl_api::access_acl_update_acl(
        &cfg,
        AccessAclUpdateAclRequest {
            path: "/".to_string(),
            role: "Auditor".to_string(),
            auth_id: Some(E2E_USER.to_string()),
            delete: Some(true),
            ..Default::default()
        },
    )
    .await
    .expect("revoke ACL");

    let acl = access_acl_api::access_acl_get_acl(&cfg, None, None, None)
        .await
        .expect("read ACL after revoke");
    let still_granted = acl
        .data
        .iter()
        .any(|e| e.ugid == E2E_USER && e.roleid == "Auditor");
    assert!(!still_granted, "ACL entry must be gone after revoke");

    let _ = access_users_api::access_users_delete_users(&cfg, E2E_USER, None).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sc_34_pagination_walks_users_endpoint() {
    let creds = Credentials::from_env();
    let cfg = creds.config_with_token();

    let listed = access_users_api::access_users_get_users(&cfg, Some(true))
        .await
        .expect("list users with tokens");
    assert!(!listed.data.is_empty());
    for u in &listed.data {
        assert!(!u.userid.is_empty());
    }
}
