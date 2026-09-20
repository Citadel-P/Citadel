use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserPreferences {
    user_id: Uuid,
    time_zone: String,
    date_time_format: UserDateTimeFormat,
    theme: UserTheme,
    updated_at: DateTime<Utc>,
}

impl UserPreferences {
    #[must_use]
    pub fn new(
        user_id: Uuid,
        time_zone: String,
        date_time_format: UserDateTimeFormat,
        theme: UserTheme,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            user_id,
            time_zone,
            date_time_format,
            theme,
            updated_at,
        }
    }

    #[must_use]
    pub const fn user_id(&self) -> Uuid {
        self.user_id
    }

    #[must_use]
    pub fn time_zone(&self) -> &str {
        &self.time_zone
    }

    #[must_use]
    pub const fn date_time_format(&self) -> UserDateTimeFormat {
        self.date_time_format
    }

    #[must_use]
    pub const fn theme(&self) -> UserTheme {
        self.theme
    }

    #[must_use]
    pub const fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    pub fn update(
        &mut self,
        time_zone: String,
        date_time_format: UserDateTimeFormat,
        theme: UserTheme,
        updated_at: DateTime<Utc>,
    ) {
        self.time_zone = time_zone;
        self.date_time_format = date_time_format;
        self.theme = theme;
        self.updated_at = updated_at;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UserDateTimeFormat {
    System,
    TwentyFourHour,
    TwelveHour,
}

impl UserDateTimeFormat {
    #[must_use]
    pub const fn as_database_str(self) -> &'static str {
        match self {
            Self::System => "System",
            Self::TwentyFourHour => "TwentyFourHour",
            Self::TwelveHour => "TwelveHour",
        }
    }

    #[must_use]
    pub fn from_database_str(value: &str) -> Option<Self> {
        match value {
            "System" => Some(Self::System),
            "TwentyFourHour" => Some(Self::TwentyFourHour),
            "TwelveHour" => Some(Self::TwelveHour),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UserTheme {
    System,
    Light,
    Dark,
}

impl UserTheme {
    #[must_use]
    pub const fn as_database_str(self) -> &'static str {
        match self {
            Self::System => "System",
            Self::Light => "Light",
            Self::Dark => "Dark",
        }
    }

    #[must_use]
    pub fn from_database_str(value: &str) -> Option<Self> {
        match value {
            "System" => Some(Self::System),
            "Light" => Some(Self::Light),
            "Dark" => Some(Self::Dark),
            _ => None,
        }
    }
}
