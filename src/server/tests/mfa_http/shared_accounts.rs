use super::*;

#[tokio::test]
#[ignore = "requires CITADEL_IDENTITY_DATABASE_URL"]
async fn shared_accounts_reject_security_mutations_and_hide_other_visitors() {
    let fixture = fixture(MfaPolicy::Optional).await;
    // An enrollment started before the deployment restriction must not complete.
    let pending = send_json(
        &fixture.app,
        Method::POST,
        "/api/v1/profile/mfa/setup",
        json!({ "password": PASSWORD }),
        Some(fixture.principal.clone()),
        None,
    )
    .await;
    assert_eq!(pending.status(), StatusCode::OK);

    let shared_identity = Arc::new(
        Arc::try_unwrap(identity(
            &fixture.pool,
            Arc::new(Argon2PasswordHasher::default()),
        ))
        .ok()
        .unwrap()
        .with_shared_user_ids([fixture.principal.subject_id]),
    );
    let app = router(
        shared_identity.clone(),
        fixture.pool.clone(),
        MfaPolicy::Optional,
    );
    let first = login(&app, &fixture.principal.name).await;
    assert_eq!(first.status(), StatusCode::OK);
    let refresh = cookie_value(&first, REFRESH_COOKIE).unwrap();
    assert_eq!(
        login(&app, &fixture.principal.name).await.status(),
        StatusCode::OK
    );
    let sessions_before = shared_identity
        .list_active_sessions(fixture.principal.subject_id)
        .await
        .unwrap();
    assert_eq!(sessions_before.len(), 2);

    let profile = send_json(
        &app,
        Method::GET,
        "/api/v1/profile",
        Value::Null,
        Some(fixture.principal.clone()),
        None,
    )
    .await;
    let profile = json_body(profile).await;
    assert_eq!(profile["authentication"]["isSharedAccount"], true);
    assert_eq!(profile["authentication"]["canChangePassword"], false);
    assert_eq!(profile["authentication"]["canUseLocalPasswordMfa"], false);

    for (method, path, body) in [
        (
            Method::PATCH,
            "/api/v1/profile".to_owned(),
            json!({"displayName":"renamed-demo"}),
        ),
        (
            Method::POST,
            "/api/v1/profile/change-password".to_owned(),
            json!({"currentPassword":PASSWORD,"newPassword":"another-correct-horse-password"}),
        ),
        (
            Method::POST,
            "/api/v1/profile/mfa/setup".to_owned(),
            json!({"password":PASSWORD}),
        ),
        (
            Method::POST,
            "/api/v1/profile/mfa/setup/confirm".to_owned(),
            json!({"code":"000001"}),
        ),
        (
            Method::POST,
            "/api/v1/profile/mfa/disable".to_owned(),
            json!({"password":PASSWORD,"code":"000002"}),
        ),
        (
            Method::POST,
            "/api/v1/profile/mfa/recovery-codes".to_owned(),
            json!({"password":PASSWORD,"code":"000003"}),
        ),
        (
            Method::DELETE,
            "/api/v1/profile/sessions".to_owned(),
            Value::Null,
        ),
        (
            Method::DELETE,
            format!("/api/v1/profile/sessions/{}", sessions_before[0].id),
            Value::Null,
        ),
    ] {
        let response = send_json(
            &app,
            method,
            &path,
            body,
            Some(fixture.principal.clone()),
            Some(&refresh),
        )
        .await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN, "{path}");
    }
    assert_eq!(
        shared_identity
            .list_active_sessions(fixture.principal.subject_id)
            .await
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        login(&app, &fixture.principal.name).await.status(),
        StatusCode::OK
    );
    let visible = send_json(
        &app,
        Method::GET,
        "/api/v1/profile/sessions",
        Value::Null,
        Some(fixture.principal.clone()),
        Some(&refresh),
    )
    .await;
    let visible = json_body(visible).await;
    assert_eq!(visible["canRevokeOtherSessions"], false);
    assert_eq!(visible["sessions"].as_array().unwrap().len(), 1);
    assert_eq!(visible["sessions"][0]["isCurrent"], true);

    let preferences = send_json(
        &app,
        Method::PATCH,
        "/api/v1/profile/preferences",
        json!({"timeZone":"Europe/Paris"}),
        Some(fixture.principal.clone()),
        None,
    )
    .await;
    assert_eq!(preferences.status(), StatusCode::OK);

    let logout = send(
        &app,
        Method::POST,
        "/api/v1/authentication/logout",
        None,
        None,
        Some(&refresh),
    )
    .await;
    assert_eq!(logout.status(), StatusCode::NO_CONTENT);
    assert_eq!(
        shared_identity
            .list_active_sessions(fixture.principal.subject_id)
            .await
            .unwrap()
            .len(),
        2
    );

    // A different account on the same instance keeps normal capabilities.
    let other = seed_administrator(&fixture.pool, &Argon2PasswordHasher::default()).await;
    let other_profile = send_json(
        &app,
        Method::GET,
        "/api/v1/profile",
        Value::Null,
        Some(other),
        None,
    )
    .await;
    let other_profile = json_body(other_profile).await;
    assert_eq!(other_profile["authentication"]["isSharedAccount"], false);
    assert_eq!(other_profile["authentication"]["canChangePassword"], true);
    assert_eq!(
        other_profile["authentication"]["canUseLocalPasswordMfa"],
        true
    );

    // Mandatory enrollment cookies issued before protection cannot bypass it.
    let old_required = router(
        fixture.identity.clone(),
        fixture.pool.clone(),
        MfaPolicy::RequiredForAllUsers,
    );
    let old_enrollment = login(&old_required, &fixture.principal.name).await;
    let setup_cookie = cookie_value(&old_enrollment, MFA_SETUP_COOKIE).unwrap();
    for (method, path, body) in [
        (Method::GET, "/api/v1/authentication/mfa/setup", None),
        (
            Method::POST,
            "/api/v1/authentication/mfa/setup/confirm",
            Some(json!({"code":"000004"})),
        ),
    ] {
        let response = send(&app, method, path, body, None, Some(&setup_cookie)).await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN, "{path}");
    }
    assert_eq!(
        mfa_settings_count(&fixture.pool, fixture.principal.subject_id).await,
        0
    );

    // Policy must never silently bypass required MFA for a shared account.
    let required = router(
        shared_identity,
        fixture.pool.clone(),
        MfaPolicy::RequiredForAllUsers,
    );
    assert_eq!(
        login(&required, &fixture.principal.name).await.status(),
        StatusCode::FORBIDDEN
    );
}
