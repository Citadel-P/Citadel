use crate::configuration::optional_name;
use crate::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AlertChannelConfiguration {
    #[serde(default, deserialize_with = "optional_name")]
    pub name: String,
    pub alert_destination: String,
    pub url: String,
    pub is_active: bool,
}

impl AlertChannelConfiguration {
    pub fn validate(&mut self) -> Result<(), AlertError> {
        self.name = self.name.trim().to_owned();
        if self.name.is_empty() {
            self.name = self.alert_destination.clone();
        }
        self.url = self.url.trim().to_owned();
        if self.name.is_empty() || self.name.chars().count() > 128 {
            return Err(AlertError::Validation(
                "Alert Channel name must contain between 1 and 128 characters.".into(),
            ));
        }
        if !matches!(
            self.alert_destination.as_str(),
            "Generic"
                | "Bark"
                | "Discord"
                | "Email"
                | "Gotify"
                | "Google_Chat"
                | "IFTTT"
                | "Join"
                | "Lark"
                | "Mattermost"
                | "Matrix"
                | "Ntfy"
                | "OpsGenie"
                | "Pushbullet"
                | "Pushover"
                | "Rocketchat"
                | "Signal"
                | "Slack"
                | "Teams"
                | "Telegram"
                | "WeCom"
                | "Zulip_Chat"
        ) {
            return Err(AlertError::Validation(
                "Alert destination is invalid.".into(),
            ));
        }
        if self.url.is_empty() || self.url.chars().count() > 4096 || !self.url.contains("://") {
            return Err(AlertError::Validation(
                "Alert Channel URL must be a non-empty Shoutrrr URL no longer than 4096 characters."
                    .into(),
            ));
        }
        Ok(())
    }
}

impl From<&AlertChannel> for AlertChannelConfiguration {
    fn from(value: &AlertChannel) -> Self {
        Self {
            name: value.name.clone(),
            alert_destination: value.alert_destination.clone(),
            url: value.url.clone(),
            is_active: value.is_active,
        }
    }
}
