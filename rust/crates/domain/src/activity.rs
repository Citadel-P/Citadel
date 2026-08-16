use std::fmt;

use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

use crate::{
    ActivityEventType, ActivityResourceType, ActivityStatus, ActorId, UserDateTimeFormat, UserTheme,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivityChangedFieldName {
    DisplayName,
    TimeZone,
    DateTimeFormat,
    Theme,
}

impl ActivityChangedFieldName {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::DisplayName => "DisplayName",
            Self::TimeZone => "TimeZone",
            Self::DateTimeFormat => "DateTimeFormat",
            Self::Theme => "Theme",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ActivityChangedField {
    #[serde(rename = "Name")]
    name: &'static str,
    #[serde(rename = "OldValue")]
    old_value: Option<String>,
    #[serde(rename = "NewValue")]
    new_value: Option<String>,
}

impl ActivityChangedField {
    #[must_use]
    pub fn display_name(old_value: String, new_value: String) -> Self {
        Self::new(
            ActivityChangedFieldName::DisplayName,
            Some(old_value),
            Some(new_value),
        )
    }

    #[must_use]
    pub fn time_zone(old_value: Option<String>, new_value: String) -> Self {
        Self::new(
            ActivityChangedFieldName::TimeZone,
            old_value,
            Some(new_value),
        )
    }

    #[must_use]
    pub fn date_time_format(old_value: UserDateTimeFormat, new_value: UserDateTimeFormat) -> Self {
        Self::new(
            ActivityChangedFieldName::DateTimeFormat,
            Some(old_value.as_database_str().to_owned()),
            Some(new_value.as_database_str().to_owned()),
        )
    }

    #[must_use]
    pub fn theme(old_value: UserTheme, new_value: UserTheme) -> Self {
        Self::new(
            ActivityChangedFieldName::Theme,
            Some(old_value.as_database_str().to_owned()),
            Some(new_value.as_database_str().to_owned()),
        )
    }

    fn new(
        name: ActivityChangedFieldName,
        old_value: Option<String>,
        new_value: Option<String>,
    ) -> Self {
        Self {
            name: name.as_str(),
            old_value,
            new_value,
        }
    }

    #[must_use]
    pub const fn name(&self) -> &str {
        self.name
    }

    #[must_use]
    pub fn old_value(&self) -> Option<&str> {
        self.old_value.as_deref()
    }

    #[must_use]
    pub fn new_value(&self) -> Option<&str> {
        self.new_value.as_deref()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "$type")]
pub enum ActivityEventInfo {
    UserProfileUpdated {
        #[serde(rename = "Changes")]
        changes: Vec<ActivityChangedField>,
    },
    UserPreferencesUpdated {
        #[serde(rename = "Changes")]
        changes: Vec<ActivityChangedField>,
    },
    UserPasswordChanged,
    UserSessionRevoked {
        #[serde(rename = "SessionId")]
        session_id: Uuid,
    },
    UserOtherSessionsRevoked {
        #[serde(rename = "Count")]
        count: i32,
    },
}

impl ActivityEventInfo {
    #[must_use]
    pub fn user_profile_updated(old_name: String, new_name: String) -> Self {
        Self::UserProfileUpdated {
            changes: vec![ActivityChangedField::display_name(old_name, new_name)],
        }
    }

    pub fn user_preferences_updated(
        changes: Vec<ActivityChangedField>,
    ) -> Result<Self, ActivityInvariantError> {
        if changes.is_empty() {
            return Err(ActivityInvariantError::EmptyChanges);
        }
        if changes
            .iter()
            .any(|change| !matches!(change.name(), "TimeZone" | "DateTimeFormat" | "Theme"))
        {
            return Err(ActivityInvariantError::InvalidChangedField);
        }
        Ok(Self::UserPreferencesUpdated { changes })
    }

    #[must_use]
    pub const fn user_password_changed() -> Self {
        Self::UserPasswordChanged
    }

    #[must_use]
    pub const fn user_session_revoked(session_id: Uuid) -> Self {
        Self::UserSessionRevoked { session_id }
    }

    pub fn user_other_sessions_revoked(count: i64) -> Result<Self, ActivityInvariantError> {
        let count = i32::try_from(count).map_err(|_| ActivityInvariantError::InvalidCount)?;
        if count <= 0 {
            return Err(ActivityInvariantError::InvalidCount);
        }
        Ok(Self::UserOtherSessionsRevoked { count })
    }

    #[must_use]
    pub const fn event_type(&self) -> ActivityEventType {
        match self {
            Self::UserProfileUpdated { .. } => ActivityEventType::UserProfileUpdated,
            Self::UserPreferencesUpdated { .. } => ActivityEventType::UserPreferencesUpdated,
            Self::UserPasswordChanged => ActivityEventType::UserPasswordChanged,
            Self::UserSessionRevoked { .. } => ActivityEventType::UserSessionRevoked,
            Self::UserOtherSessionsRevoked { .. } => ActivityEventType::UserOtherSessionsRevoked,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivityEvent {
    id: Uuid,
    platform_id: Option<Uuid>,
    resource_id: Uuid,
    resource_name: String,
    resource_type: ActivityResourceType,
    status: ActivityStatus,
    event_type: ActivityEventType,
    info: ActivityEventInfo,
    created_by_actor_id: ActorId,
    created_at: DateTime<Utc>,
}

impl ActivityEvent {
    pub fn new_user_event(
        resource_id: Uuid,
        resource_name: String,
        actor_id: ActorId,
        info: ActivityEventInfo,
        created_at: DateTime<Utc>,
    ) -> Result<Self, ActivityInvariantError> {
        if resource_id.is_nil() {
            return Err(ActivityInvariantError::MissingResourceId);
        }
        if actor_id.value().is_nil() {
            return Err(ActivityInvariantError::MissingActorId);
        }
        let resource_name = resource_name.trim();
        if resource_name.is_empty() {
            return Err(ActivityInvariantError::MissingResourceName);
        }
        let event_type = info.event_type();
        let resource_type = event_type.resource_type();
        if resource_type != ActivityResourceType::User {
            return Err(ActivityInvariantError::MismatchedResourceType);
        }
        Ok(Self {
            id: Uuid::now_v7(),
            platform_id: None,
            resource_id,
            resource_name: resource_name.to_owned(),
            resource_type,
            status: ActivityStatus::Success,
            event_type,
            info,
            created_by_actor_id: actor_id,
            created_at,
        })
    }

    #[must_use]
    pub const fn id(&self) -> Uuid {
        self.id
    }

    #[must_use]
    pub const fn platform_id(&self) -> Option<Uuid> {
        self.platform_id
    }

    #[must_use]
    pub const fn resource_id(&self) -> Uuid {
        self.resource_id
    }

    #[must_use]
    pub fn resource_name(&self) -> &str {
        &self.resource_name
    }

    #[must_use]
    pub const fn resource_type(&self) -> ActivityResourceType {
        self.resource_type
    }

    #[must_use]
    pub const fn status(&self) -> ActivityStatus {
        self.status
    }

    #[must_use]
    pub const fn event_type(&self) -> ActivityEventType {
        self.event_type
    }

    #[must_use]
    pub const fn info(&self) -> &ActivityEventInfo {
        &self.info
    }

    #[must_use]
    pub const fn created_by_actor_id(&self) -> ActorId {
        self.created_by_actor_id
    }

    #[must_use]
    pub const fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivityInvariantError {
    MissingResourceId,
    MissingActorId,
    MissingResourceName,
    EmptyChanges,
    InvalidChangedField,
    InvalidCount,
    MismatchedResourceType,
}

impl fmt::Display for ActivityInvariantError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::MissingResourceId => "activity resource ID is required",
            Self::MissingActorId => "activity actor ID is required",
            Self::MissingResourceName => "activity resource name is required",
            Self::EmptyChanges => "activity changes cannot be empty",
            Self::InvalidChangedField => "activity contains a non-allow-listed changed field",
            Self::InvalidCount => "activity count must be a positive 32-bit integer",
            Self::MismatchedResourceType => "activity event and resource types do not match",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for ActivityInvariantError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_activity_derives_its_discriminators() {
        let event = ActivityEvent::new_user_event(
            Uuid::now_v7(),
            "Owner".to_owned(),
            ActorId::new(Uuid::now_v7()),
            ActivityEventInfo::user_profile_updated("Old owner".to_owned(), "Owner".to_owned()),
            Utc::now(),
        )
        .unwrap();

        assert_eq!(event.event_type(), ActivityEventType::UserProfileUpdated);
        assert_eq!(event.resource_type(), ActivityResourceType::User);
        assert_eq!(event.status(), ActivityStatus::Success);
    }

    #[test]
    fn empty_preference_changes_are_rejected() {
        assert_eq!(
            ActivityEventInfo::user_preferences_updated(Vec::new()),
            Err(ActivityInvariantError::EmptyChanges)
        );
    }

    #[test]
    fn revoked_other_sessions_requires_a_positive_count() {
        assert_eq!(
            ActivityEventInfo::user_other_sessions_revoked(0),
            Err(ActivityInvariantError::InvalidCount)
        );
    }
}
