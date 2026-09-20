use super::*;

pub struct ProfileService {
    store: Arc<dyn ProfileRepository>,
    identity: Arc<IdentityService>,
    clock: Arc<dyn Clock>,
}

impl ProfileService {
    #[must_use]
    pub fn new(
        store: Arc<dyn ProfileRepository>,
        identity: Arc<IdentityService>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            store,
            identity,
            clock,
        }
    }

    pub async fn get(
        &self,
        principal: &ActorPrincipal,
    ) -> Result<CurrentProfileDetails, IdentityError> {
        require_human(principal)?;
        let record = self
            .store
            .get_current(principal.subject_id)
            .await?
            .ok_or(IdentityError::NotFound)?;
        self.map(principal, record).await
    }

    pub async fn update(
        &self,
        principal: &ActorPrincipal,
        request: UpdateCurrentProfile,
    ) -> Result<CurrentProfileDetails, IdentityError> {
        require_human(principal)?;
        validate_name(&request.display_name)?;
        let user = self
            .store
            .get_user(principal.subject_id)
            .await?
            .ok_or(IdentityError::NotFound)?;
        let display_name = request.display_name.trim();
        let record = if user.name() == display_name {
            self.store
                .get_current(principal.subject_id)
                .await?
                .ok_or(IdentityError::NotFound)?
        } else {
            self.store
                .rename_user(
                    principal.subject_id,
                    display_name,
                    principal.actor_id,
                    self.clock.now(),
                )
                .await?
        };
        self.map(principal, record).await
    }

    pub async fn get_preferences(
        &self,
        principal: &ActorPrincipal,
    ) -> Result<UserPreferencesDetails, IdentityError> {
        require_human(principal)?;
        Ok(self
            .store
            .get_preferences(principal.subject_id)
            .await?
            .map_or_else(default_preferences, persisted_preferences))
    }

    pub async fn patch_preferences(
        &self,
        principal: &ActorPrincipal,
        request: PatchUserPreferences,
    ) -> Result<UserPreferencesDetails, IdentityError> {
        require_human(principal)?;
        if matches!(&request.time_zone, PatchField::Missing)
            && matches!(&request.date_time_format, PatchField::Missing)
            && matches!(&request.theme, PatchField::Missing)
        {
            return Err(IdentityError::Validation(
                "At least one preference field is required.".to_owned(),
            ));
        }
        reject_null(&request.time_zone, "timeZone")?;
        reject_null(&request.date_time_format, "dateTimeFormat")?;
        reject_null(&request.theme, "theme")?;

        let time_zone = match &request.time_zone {
            PatchField::Missing => None,
            PatchField::Value(value) => Some(validate_time_zone(value)?),
            PatchField::Null => unreachable!("null fields were rejected"),
        };
        let date_time_format = match request.date_time_format {
            PatchField::Missing => None,
            PatchField::Value(value) => Some(value),
            PatchField::Null => unreachable!("null fields were rejected"),
        };
        let theme = match request.theme {
            PatchField::Missing => None,
            PatchField::Value(value) => Some(value),
            PatchField::Null => unreachable!("null fields were rejected"),
        };
        self.store
            .patch_preferences(
                principal.subject_id,
                &UserPreferencesUpdate {
                    time_zone,
                    date_time_format,
                    theme,
                },
                self.clock.now(),
                principal.actor_id,
            )
            .await
            .map(persisted_preferences)
    }

    pub async fn list_sessions(
        &self,
        principal: &ActorPrincipal,
        refresh_token: Option<&str>,
    ) -> Result<UserSessionsDetails, IdentityError> {
        require_human(principal)?;
        let current_session_id = self
            .identity
            .resolve_current_session(principal.subject_id, refresh_token)
            .await?;
        let records = self
            .identity
            .list_active_sessions(principal.subject_id)
            .await?;
        let current_session_id =
            current_session_id.filter(|id| records.iter().any(|session| session.id == *id));
        let mut sessions = records
            .into_iter()
            .map(|session| map_session(session, current_session_id))
            .collect::<Vec<_>>();
        sessions.sort_by(|left, right| {
            right
                .is_current
                .cmp(&left.is_current)
                .then_with(|| right.last_seen_at.cmp(&left.last_seen_at))
                .then_with(|| right.created_at.cmp(&left.created_at))
        });
        Ok(UserSessionsDetails {
            sessions,
            can_revoke_other_sessions: current_session_id.is_some(),
        })
    }

    pub async fn revoke_session(
        &self,
        principal: &ActorPrincipal,
        refresh_token: Option<&str>,
        session_id: Uuid,
    ) -> Result<(), IdentityError> {
        require_human(principal)?;
        let current_session_id = self
            .identity
            .resolve_current_session(principal.subject_id, refresh_token)
            .await?;
        if self
            .identity
            .revoke_owned_session(
                session_id,
                principal.subject_id,
                current_session_id,
                principal.actor_id,
                self.clock.now(),
            )
            .await?
        {
            Ok(())
        } else {
            Err(IdentityError::NotFound)
        }
    }

    pub async fn revoke_other_sessions(
        &self,
        principal: &ActorPrincipal,
        refresh_token: Option<&str>,
    ) -> Result<RevokeOtherProfileSessionsDetails, IdentityError> {
        require_human(principal)?;
        let current_session_id = self
            .identity
            .resolve_current_session(principal.subject_id, refresh_token)
            .await?
            .ok_or_else(current_session_required)?;
        let count = self
            .identity
            .revoke_other_sessions(principal.subject_id, current_session_id, principal.actor_id)
            .await?
            .ok_or_else(current_session_required)?;
        Ok(RevokeOtherProfileSessionsDetails { count })
    }

    pub async fn change_password(
        &self,
        principal: &ActorPrincipal,
        refresh_token: Option<&str>,
        request: ChangeCurrentPassword,
    ) -> Result<(), IdentityError> {
        require_human(principal)?;
        if request.current_password.chars().count() > MAXIMUM_PASSWORD_CHARACTERS {
            return Err(current_password_incorrect());
        }

        let profile = self
            .store
            .get_current(principal.subject_id)
            .await?
            .ok_or(IdentityError::NotFound)?;
        if profile.oidc_provider_id.is_some() {
            return Err(externally_managed_password());
        }
        let user = self
            .store
            .get_user(principal.subject_id)
            .await?
            .ok_or(IdentityError::NotFound)?;
        let Some(current_hash) = user.password_hash() else {
            return Err(current_password_incorrect());
        };
        if !self
            .identity
            .verify_password(request.current_password, current_hash.to_owned())
            .await?
        {
            return Err(current_password_incorrect());
        }

        validate_password(&request.new_password, Some(user.name()), Some(user.email()))?;
        let new_hash = self.identity.hash_password(request.new_password).await?;
        let current_session_id = self
            .identity
            .resolve_current_session(principal.subject_id, refresh_token)
            .await?;
        let changed_at = self.clock.now();
        match self
            .store
            .change_password(
                principal.subject_id,
                current_hash,
                &new_hash,
                current_session_id,
                changed_at,
                principal.actor_id,
            )
            .await?
        {
            PasswordChangeOutcome::Changed => Ok(()),
            PasswordChangeOutcome::CurrentPasswordMismatch => Err(current_password_incorrect()),
            PasswordChangeOutcome::ExternallyManaged => Err(externally_managed_password()),
        }
    }

    async fn map(
        &self,
        principal: &ActorPrincipal,
        record: CurrentProfileRecord,
    ) -> Result<CurrentProfileDetails, IdentityError> {
        if record.actor_id != principal.actor_id {
            return Err(IdentityError::NotFound);
        }
        let authorization = self.identity.authorization_snapshot(principal).await?;
        let is_oidc = record.oidc_provider_id.is_some();
        Ok(CurrentProfileDetails {
            id: record.id,
            display_name: record.display_name,
            email: record.email,
            authentication: CurrentProfileAuthenticationDetails {
                r#type: if is_oidc {
                    CurrentProfileAuthenticationType::Oidc
                } else {
                    CurrentProfileAuthenticationType::Local
                },
                label: if is_oidc {
                    format!(
                        "Managed by {}",
                        record
                            .oidc_provider_name
                            .as_deref()
                            .unwrap_or("identity provider")
                    )
                } else {
                    "Local account".to_owned()
                },
                can_change_password: !is_oidc,
                can_use_local_password_mfa: record.has_local_password,
                oidc_provider_id: record.oidc_provider_id,
                oidc_provider_name: record.oidc_provider_name,
            },
            authorization: CurrentProfileAuthorizationDetails {
                is_administrator: principal.is_administrator(),
                alert_rules: combined_capabilities(
                    &authorization,
                    ResourceType::Alert,
                    ResourceType::AlertChannel,
                ),
                bindings: capabilities(&authorization, ResourceType::Binding),
                tags: capabilities(&authorization, ResourceType::Tag),
            },
            created_at: record.created_at,
            direct_roles: record.direct_roles,
            teams: record.teams,
        })
    }
}

pub(super) fn default_preferences() -> UserPreferencesDetails {
    UserPreferencesDetails {
        time_zone: None,
        date_time_format: UserDateTimeFormat::System,
        theme: UserTheme::System,
        is_persisted: false,
    }
}

pub(super) fn persisted_preferences(preferences: UserPreferences) -> UserPreferencesDetails {
    UserPreferencesDetails {
        time_zone: Some(preferences.time_zone().to_owned()),
        date_time_format: preferences.date_time_format(),
        theme: preferences.theme(),
        is_persisted: true,
    }
}

pub(super) fn reject_null<T>(field: &PatchField<T>, name: &str) -> Result<(), IdentityError> {
    if matches!(field, PatchField::Null) {
        Err(IdentityError::Validation(format!("{name} cannot be null.")))
    } else {
        Ok(())
    }
}

pub(super) fn validate_time_zone(value: &str) -> Result<String, IdentityError> {
    let value = value.trim();
    if value.is_empty() || value.parse::<chrono_tz::Tz>().is_err() {
        return Err(IdentityError::Validation(
            "TimeZone must be a valid IANA timezone.".to_owned(),
        ));
    }
    Ok(value.to_owned())
}

pub(super) fn current_session_required() -> IdentityError {
    IdentityError::Validation("Current refresh session could not be resolved.".to_owned())
}

pub(super) fn current_password_incorrect() -> IdentityError {
    IdentityError::Validation("Current password is incorrect.".to_owned())
}

pub(super) fn externally_managed_password() -> IdentityError {
    IdentityError::Validation("Password changes are managed by the identity provider.".to_owned())
}

pub(super) fn map_session(
    session: UserSessionRecord,
    current_session_id: Option<Uuid>,
) -> UserSessionSummaryDetails {
    UserSessionSummaryDetails {
        id: session.id,
        display_name: session_display_name(session.user_agent.as_deref()),
        user_agent: session.user_agent,
        ip_address: session.ip_address,
        created_at: session.created_at,
        last_seen_at: session.last_seen_at,
        expires_at: session.expires_at,
        is_current: current_session_id == Some(session.id),
    }
}

pub(super) fn session_display_name(user_agent: Option<&str>) -> String {
    let Some(user_agent) = user_agent.filter(|value| !value.trim().is_empty()) else {
        return "Unknown browser".to_owned();
    };
    let normalized = user_agent.to_ascii_lowercase();
    let browser = if normalized.contains("edg/") {
        "Edge"
    } else if normalized.contains("chrome/") {
        "Chrome"
    } else if normalized.contains("firefox/") {
        "Firefox"
    } else if normalized.contains("safari/") {
        "Safari"
    } else if normalized.contains("curl/") {
        "curl"
    } else {
        "Unknown browser"
    };
    let operating_system = if normalized.contains("windows") {
        Some("Windows")
    } else if normalized.contains("mac os x") {
        Some("macOS")
    } else if normalized.contains("android") {
        Some("Android")
    } else if normalized.contains("iphone") || normalized.contains("ipad") {
        Some("iOS")
    } else if normalized.contains("linux") {
        Some("Linux")
    } else {
        None
    };
    operating_system.map_or_else(
        || browser.to_owned(),
        |operating_system| format!("{browser} on {operating_system}"),
    )
}

pub(super) fn require_human(principal: &ActorPrincipal) -> Result<(), IdentityError> {
    if principal.is_human() {
        Ok(())
    } else {
        Err(IdentityError::Forbidden)
    }
}

pub(super) fn capabilities(
    authorization: &AuthorizationSnapshot,
    resource_type: ResourceType,
) -> ResourceCapabilities {
    ResourceCapabilities {
        can_read: authorization.permits(resource_type, PermissionLevel::Read, None),
        can_write: authorization.permits(resource_type, PermissionLevel::Write, None),
        can_execute: authorization.permits(resource_type, PermissionLevel::Execute, None),
    }
}

pub(super) fn combined_capabilities(
    authorization: &AuthorizationSnapshot,
    first: ResourceType,
    second: ResourceType,
) -> ResourceCapabilities {
    let first = capabilities(authorization, first);
    let second = capabilities(authorization, second);
    ResourceCapabilities {
        can_read: first.can_read || second.can_read,
        can_write: first.can_write || second.can_write,
        can_execute: first.can_execute || second.can_execute,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use citadel_primitives::{ActorId, SpecificPermission};

    use crate::PermissionGrant;

    #[test]
    fn profile_capabilities_combine_alert_resources() {
        let authorization = AuthorizationSnapshot {
            actor_id: ActorId::new(Uuid::now_v7()),
            enabled: true,
            direct_and_team_permissions: vec![PermissionGrant {
                resource_type: ResourceType::AlertChannel,
                level: PermissionLevel::Write,
                specifics: SpecificPermission::Use.into(),
            }],
        };

        let result = combined_capabilities(
            &authorization,
            ResourceType::Alert,
            ResourceType::AlertChannel,
        );

        assert!(result.can_read);
        assert!(result.can_write);
        assert!(!result.can_execute);
    }

    #[test]
    fn time_zone_validation_accepts_iana_names_and_rejects_platform_names() {
        assert_eq!(
            validate_time_zone(" Europe/Paris ").unwrap(),
            "Europe/Paris"
        );
        assert_eq!(validate_time_zone("UTC").unwrap(), "UTC");
        assert!(validate_time_zone("Romance Standard Time").is_err());
        assert!(validate_time_zone("Not/AZone").is_err());
    }

    #[test]
    fn session_display_names_match_the_existing_profile_contract() {
        assert_eq!(
            session_display_name(Some(
                "Mozilla/5.0 (Windows NT 10.0) AppleWebKit Chrome/140.0 Safari/537.36"
            )),
            "Chrome on Windows"
        );
        assert_eq!(session_display_name(Some("curl/8.16.0")), "curl");
        assert_eq!(session_display_name(None), "Unknown browser");
    }
}
