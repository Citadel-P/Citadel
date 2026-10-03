//! Shared domain control vocabulary; persistence must reject unknown states.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ResourceControlState {
    #[default]
    Idle,
    Queued,
    Processing,
}
impl ResourceControlState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Idle => "Idle",
            Self::Queued => "Queued",
            Self::Processing => "Processing",
        }
    }
    pub const fn is_idle(self) -> bool {
        matches!(self, Self::Idle)
    }
}
impl std::fmt::Display for ResourceControlState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}
impl std::str::FromStr for ResourceControlState {
    type Err = String;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "Idle" => Ok(Self::Idle),
            "Queued" => Ok(Self::Queued),
            "Processing" => Ok(Self::Processing),
            _ => Err(format!("Unknown resource control state '{value}'.")),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unknown_persisted_state_is_rejected() {
        assert!("invalid".parse::<ResourceControlState>().is_err());
    }
}
