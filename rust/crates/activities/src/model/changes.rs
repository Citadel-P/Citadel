use super::*;
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
    pub fn date_time_format(old_value: &str, new_value: &str) -> Self {
        Self::new(
            ActivityChangedFieldName::DateTimeFormat,
            Some(old_value.to_owned()),
            Some(new_value.to_owned()),
        )
    }

    #[must_use]
    pub fn theme(old_value: &str, new_value: &str) -> Self {
        Self::new(
            ActivityChangedFieldName::Theme,
            Some(old_value.to_owned()),
            Some(new_value.to_owned()),
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
