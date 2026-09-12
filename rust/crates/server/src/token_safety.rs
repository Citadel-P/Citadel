//! Credential restrictions shared by HTTP and websocket authentication.
use citadel_domain::AuthenticatedPrincipalType;
use citadel_identity::{ActorPrincipal, AuthenticatedBearer, IdentityError, IdentityService};

pub(crate) fn authorize_http(auth: &AuthenticatedBearer, path: &str) -> Result<(), IdentityError> {
    let path = path.to_ascii_lowercase();
    if auth.principal.principal_type == AuthenticatedPrincipalType::ServiceAccount
        && ["/api/v1/authentication", "/api/v1/profile", "/hubs"]
            .iter()
            .any(|prefix| starts_with_segments(&path, prefix))
    {
        return Err(IdentityError::Forbidden);
    }
    if auth.automation_run_id.is_some()
        && ([
            "/api/v1/authentication",
            "/api/v1/automation",
            "/api/v1/resourcebindings/secrets",
            "/api/v1/resourcebindings/secret-providers",
        ]
        .iter()
        .any(|prefix| starts_with_segments(&path, prefix))
            || path.contains("/terminal")
            || path.contains("/exec"))
    {
        return Err(IdentityError::Forbidden);
    }
    Ok(())
}

fn starts_with_segments(path: &str, prefix: &str) -> bool {
    path.strip_prefix(prefix)
        .is_some_and(|tail| tail.is_empty() || tail.starts_with('/'))
}

pub(crate) async fn authenticate_realtime(
    identity: &IdentityService,
    token: &str,
) -> Result<ActorPrincipal, IdentityError> {
    // Retain the run claim until after the check. authenticate_bearer() alone
    // discards it and could turn a restricted run token into an interactive user.
    let authenticated = identity.authenticate_bearer_context(token).await?;
    authorize_realtime(&authenticated)?;
    Ok(authenticated.principal)
}

fn authorize_realtime(auth: &AuthenticatedBearer) -> Result<(), IdentityError> {
    // Realtime is a browser connection with both subscriptions and execution
    // commands. Service accounts and automation runs use the scoped HTTP API.
    if !auth.principal.is_human() || auth.automation_run_id.is_some() {
        return Err(IdentityError::Forbidden);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use citadel_domain::ActorId;
    use uuid::Uuid;

    fn authenticated(kind: AuthenticatedPrincipalType, run: bool) -> AuthenticatedBearer {
        AuthenticatedBearer {
            principal: ActorPrincipal {
                subject_id: Uuid::now_v7(),
                actor_id: ActorId::new(Uuid::now_v7()),
                name: "fixture".into(),
                principal_type: kind,
                credential_id: None,
                roles: vec!["Admin".into()],
            },
            automation_run_id: run.then(Uuid::now_v7),
        }
    }

    #[test]
    fn service_accounts_cannot_enter_browser_routes_even_with_admin_role() {
        let auth = authenticated(AuthenticatedPrincipalType::ServiceAccount, false);
        for path in [
            "/api/v1/authentication",
            "/API/V1/AUTHENTICATION/login",
            "/api/v1/authentication/refresh",
            "/api/v1/profile/sessions",
            "/HUBS",
            "/hubs/citadel",
        ] {
            assert!(
                matches!(authorize_http(&auth, path), Err(IdentityError::Forbidden)),
                "{path}"
            );
        }
        for path in [
            "/api/v1/platforms",
            "/api/v1/stacks/id/apply",
            "/api/v1/automation/actions",
            "/api/v1/serviceAccounts",
            "/api/v1/profile-other",
            "/hubs-other",
        ] {
            assert!(authorize_http(&auth, path).is_ok(), "{path}");
        }
    }

    #[test]
    fn automation_restrictions_apply_to_user_and_service_account_runs() {
        for kind in [
            AuthenticatedPrincipalType::User,
            AuthenticatedPrincipalType::ServiceAccount,
        ] {
            let auth = authenticated(kind, true);
            for path in [
                "/api/v1/authentication/logout",
                "/API/V1/AUTOMATION/actions",
                "/api/v1/resourceBindings/secrets",
                "/api/v1/resourceBindings/SECRET-PROVIDERS/id",
                "/api/v1/platforms/id/containers/id/Terminal",
                "/api/v1/containers/id/exec/start",
                "/api/v1/containers/id/exec-resize",
            ] {
                assert!(
                    matches!(authorize_http(&auth, path), Err(IdentityError::Forbidden)),
                    "{path}"
                );
            }
            for path in [
                "/api/v1/platforms",
                "/api/v1/stacks/id/apply",
                "/api/v1/containers/id/logs",
                "/api/v1/automation-other",
                "/api/v1/resourceBindings/secrets-other",
            ] {
                assert!(authorize_http(&auth, path).is_ok(), "{path}");
            }
        }
    }

    #[test]
    fn realtime_requires_a_normal_user_token() {
        for (kind, run, allowed) in [
            (AuthenticatedPrincipalType::User, false, true),
            (AuthenticatedPrincipalType::User, true, false),
            (AuthenticatedPrincipalType::ServiceAccount, false, false),
            (AuthenticatedPrincipalType::ServiceAccount, true, false),
        ] {
            assert_eq!(
                authorize_realtime(&authenticated(kind, run)).is_ok(),
                allowed
            );
        }
        let human = authenticated(AuthenticatedPrincipalType::User, false);
        for path in [
            "/api/v1/authentication/login",
            "/api/v1/profile",
            "/hubs",
            "/api/v1/automation",
            "/api/v1/resourceBindings/secrets",
            "/api/v1/containers/id/terminal",
        ] {
            assert!(authorize_http(&human, path).is_ok());
        }
    }
}
