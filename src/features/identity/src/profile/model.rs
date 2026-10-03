use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserPreferences {
    user_id: Uuid,
    time_zone: String,
    date_time_format: UserDateTimeFormat,
    theme: UserTheme,
    appearance: UserAppearance,
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
            appearance: UserAppearance::default(),
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
    pub fn with_appearance(mut self, appearance: UserAppearance) -> Self {
        self.appearance = appearance;
        self
    }

    #[must_use]
    pub const fn theme_color(&self) -> UserThemeColor {
        self.appearance.theme_color
    }

    #[must_use]
    pub const fn font(&self) -> UserUiFont {
        self.appearance.font
    }

    #[must_use]
    pub const fn radius(&self) -> UserUiRadius {
        self.appearance.radius
    }

    #[must_use]
    pub const fn content_layout(&self) -> UserContentLayout {
        self.appearance.content_layout
    }

    #[must_use]
    pub const fn density(&self) -> UserUiDensity {
        self.appearance.density
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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct UserAppearance {
    pub theme_color: UserThemeColor,
    pub font: UserUiFont,
    pub radius: UserUiRadius,
    pub content_layout: UserContentLayout,
    pub density: UserUiDensity,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum UserThemeColor {
    #[default]
    Neutral,
    Blue,
    Indigo,
    Violet,
    Emerald,
    Yellow,
    Orange,
    Rose,
}
impl UserThemeColor {
    #[must_use]
    pub const fn as_database_str(self) -> &'static str {
        match self {
            Self::Neutral => "Neutral",
            Self::Blue => "Blue",
            Self::Indigo => "Indigo",
            Self::Violet => "Violet",
            Self::Emerald => "Emerald",
            Self::Yellow => "Yellow",
            Self::Orange => "Orange",
            Self::Rose => "Rose",
        }
    }
    #[must_use]
    pub fn from_database_str(value: &str) -> Option<Self> {
        match value {
            "Neutral" => Some(Self::Neutral),
            "Blue" => Some(Self::Blue),
            "Indigo" => Some(Self::Indigo),
            "Violet" => Some(Self::Violet),
            "Emerald" => Some(Self::Emerald),
            "Yellow" => Some(Self::Yellow),
            "Orange" => Some(Self::Orange),
            "Rose" => Some(Self::Rose),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum UserUiFont {
    #[default]
    Geist,
    Inter,
    IbmPlexSans,
    SourceSans3,
    System,
}
impl UserUiFont {
    #[must_use]
    pub const fn as_database_str(self) -> &'static str {
        match self {
            Self::Geist => "Geist",
            Self::Inter => "Inter",
            Self::IbmPlexSans => "IbmPlexSans",
            Self::SourceSans3 => "SourceSans3",
            Self::System => "System",
        }
    }
    #[must_use]
    pub fn from_database_str(value: &str) -> Option<Self> {
        match value {
            "Geist" => Some(Self::Geist),
            "Inter" => Some(Self::Inter),
            "IbmPlexSans" => Some(Self::IbmPlexSans),
            "SourceSans3" => Some(Self::SourceSans3),
            "System" => Some(Self::System),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum UserUiRadius {
    #[default]
    None,
    Small,
    Medium,
    Large,
}
impl UserUiRadius {
    #[must_use]
    pub const fn as_database_str(self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Small => "Small",
            Self::Medium => "Medium",
            Self::Large => "Large",
        }
    }
    #[must_use]
    pub fn from_database_str(value: &str) -> Option<Self> {
        match value {
            "None" => Some(Self::None),
            "Small" => Some(Self::Small),
            "Medium" => Some(Self::Medium),
            "Large" => Some(Self::Large),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum UserContentLayout {
    Compact,
    Wide,
    #[default]
    Full,
}
impl UserContentLayout {
    #[must_use]
    pub const fn as_database_str(self) -> &'static str {
        match self {
            Self::Compact => "Compact",
            Self::Wide => "Wide",
            Self::Full => "Full",
        }
    }
    #[must_use]
    pub fn from_database_str(value: &str) -> Option<Self> {
        match value {
            "Compact" => Some(Self::Compact),
            "Wide" => Some(Self::Wide),
            "Full" => Some(Self::Full),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum UserUiDensity {
    #[default]
    Compact,
    Comfortable,
}
impl UserUiDensity {
    #[must_use]
    pub const fn as_database_str(self) -> &'static str {
        match self {
            Self::Compact => "Compact",
            Self::Comfortable => "Comfortable",
        }
    }
    #[must_use]
    pub fn from_database_str(value: &str) -> Option<Self> {
        match value {
            "Compact" => Some(Self::Compact),
            "Comfortable" => Some(Self::Comfortable),
            _ => None,
        }
    }
}
