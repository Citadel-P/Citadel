use super::ConfigError;
use uuid::Uuid;

const SETTING: &str = "CITADEL_SHARED_USER_IDS";

pub(super) fn from_env() -> Result<Vec<Uuid>, ConfigError> {
    match std::env::var(SETTING) {
        Ok(value) => parse(&value),
        Err(std::env::VarError::NotPresent) => Ok(Vec::new()),
        Err(_) => Err(invalid()),
    }
}

fn invalid() -> ConfigError {
    ConfigError::Invalid {
        name: SETTING,
        message: "must be a comma-separated list of user UUIDs".into(),
    }
}

fn parse(value: &str) -> Result<Vec<Uuid>, ConfigError> {
    if value.trim().is_empty() {
        return Ok(Vec::new());
    }
    value
        .split(',')
        .map(|id| Uuid::parse_str(id.trim()).map_err(|_| invalid()))
        .collect()
}
