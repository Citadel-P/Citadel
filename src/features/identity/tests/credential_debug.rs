use citadel_identity::{ChangeCurrentPassword, InitializeCitadel, Login, MfaVerificationInput};

#[test]
fn credential_debug_redacts_plaintext_without_changing_serialization() {
    let login = Login {
        email_or_name: "public-name".into(),
        password: "SYNTHETIC_PASSWORD".into(),
    };
    assert!(format!("{login:?}").contains("public-name"));
    assert!(!format!("{login:?}").contains("SYNTHETIC_PASSWORD"));
    assert_eq!(
        serde_json::to_value(&login).unwrap()["password"],
        "SYNTHETIC_PASSWORD"
    );
    let initialize = InitializeCitadel {
        name: "name".into(),
        email: "email".into(),
        password: "SYNTHETIC_PASSWORD".into(),
        update_checks_enabled: None,
        mfa_policy: None,
    };
    assert!(!format!("{initialize:?}").contains("SYNTHETIC_PASSWORD"));
    let change = ChangeCurrentPassword {
        current_password: "SYNTHETIC_OLD".into(),
        new_password: "SYNTHETIC_NEW".into(),
    };
    assert!(!format!("{change:?}").contains("SYNTHETIC_"));
    let mfa = MfaVerificationInput {
        code: Some("SYNTHETIC_CODE".into()),
        recovery_code: Some("SYNTHETIC_RECOVERY".into()),
    };
    assert!(!format!("{mfa:?}").contains("SYNTHETIC_"));
}

#[test]
fn mfa_one_time_outputs_are_delivered_but_never_debugged() {
    use citadel_identity::{
        MandatoryMfaSetupCompleteDetails, MandatoryMfaSetupDetails, MfaVerificationDetails,
        ProfileMfaRecoveryCodesDetails, ProfileMfaSetupDetails,
    };
    fn check(value: impl std::fmt::Debug + serde::Serialize) {
        assert!(!format!("{value:?}").contains("SYNTHETIC_"));
        assert!(
            serde_json::to_string(&value)
                .unwrap()
                .contains("SYNTHETIC_")
        );
    }
    check(MandatoryMfaSetupDetails {
        secret: "SYNTHETIC_SECRET".into(),
        otp_auth_uri: "SYNTHETIC_URI".into(),
        expires_at: chrono::Utc::now(),
    });
    check(ProfileMfaSetupDetails {
        secret: "SYNTHETIC_SECRET".into(),
        otp_auth_uri: "SYNTHETIC_URI".into(),
        expires_at: chrono::Utc::now(),
    });
    check(MandatoryMfaSetupCompleteDetails {
        access_token: "SYNTHETIC_TOKEN".into(),
        recovery_codes: vec!["SYNTHETIC_RECOVERY".into()],
    });
    check(MfaVerificationDetails {
        access_token: "SYNTHETIC_TOKEN".into(),
    });
    check(ProfileMfaRecoveryCodesDetails {
        enabled: true,
        recovery_codes: vec!["SYNTHETIC_RECOVERY".into()],
    });
}

#[test]
fn oidc_and_totp_credentials_are_redacted_without_discarding_values() {
    use citadel_identity::{CreateOidcProvider, OidcLoginState, PatchOidcProvider, TotpSetup};
    let create: CreateOidcProvider = serde_json::from_value(serde_json::json!({
        "name":"oidc", "displayName":"OIDC", "issuer":"https://issuer.test", "clientId":"public-id",
        "clientSecret":"SYNTHETIC_SECRET", "enabled":true, "autoProvisionUsers":false,
        "allowEmailAutoLink":false, "requireEmailVerified":true
    }))
    .unwrap();
    let patch: PatchOidcProvider =
        serde_json::from_value(serde_json::json!({"clientSecret":"SYNTHETIC_SECRET"})).unwrap();
    assert!(!format!("{create:?} {patch:?}").contains("SYNTHETIC_"));
    assert_eq!(create.client_secret.as_deref(), Some("SYNTHETIC_SECRET"));
    let now = chrono::Utc::now();
    let state = OidcLoginState {
        id: uuid::Uuid::now_v7(),
        provider_id: uuid::Uuid::now_v7(),
        state_hash: "hash".into(),
        nonce: "SYNTHETIC_NONCE".into(),
        code_verifier: "SYNTHETIC_VERIFIER".into(),
        return_url: "/".into(),
        created_at: now,
        expires_at: now,
    };
    let totp = TotpSetup {
        secret: "SYNTHETIC_SECRET".into(),
        otp_auth_uri: "SYNTHETIC_URI".into(),
        expires_at: now,
    };
    assert!(!format!("{state:?} {totp:?}").contains("SYNTHETIC_"));
    assert_eq!(totp.secret, "SYNTHETIC_SECRET");
    assert_eq!(state.code_verifier, "SYNTHETIC_VERIFIER");
}

#[test]
fn authentication_urls_and_mfa_cookie_identifiers_are_not_debug_output() {
    use citadel_identity::{
        BrowserAuthenticationAction, MfaChallenge, MfaSetupSession, OidcLoginStart,
    };
    let id = uuid::Uuid::now_v7();
    let user_id = uuid::Uuid::now_v7();
    let now = chrono::Utc::now();
    let setup = MfaSetupSession {
        id,
        user_id,
        protected_totp_secret: "ciphertext".into(),
        expires_at: now,
        consumed_at: None,
        created_at: now,
    };
    let challenge = MfaChallenge {
        id,
        user_id,
        expires_at: now,
        failed_attempts: 0,
        consumed_at: None,
        created_at: now,
    };
    let action = BrowserAuthenticationAction::EnrollMfa {
        setup_session_id: id,
        expires_at: now,
    };
    let verify = BrowserAuthenticationAction::VerifyMfa {
        challenge_id: id,
        expires_at: now,
    };
    assert!(!format!("{setup:?} {challenge:?} {action:?} {verify:?}").contains(&id.to_string()));
    let start = OidcLoginStart {
        state: "SYNTHETIC_STATE".into(),
        browser_token: "SYNTHETIC_BROWSER_TOKEN".into(),
        authorization_url: "https://issuer.test?state=SYNTHETIC_STATE".into(),
    };
    assert!(!format!("{start:?}").contains("SYNTHETIC_STATE"));
    assert!(start.authorization_url.contains("SYNTHETIC_STATE"));
    assert!(!format!("{start:?}").contains("SYNTHETIC_BROWSER_TOKEN"));
}
