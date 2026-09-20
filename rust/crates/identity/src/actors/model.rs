use citadel_primitives::ActorId;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
pub const SYSTEM_ACTOR_ID: Uuid = Uuid::from_u128(1);

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActorPrincipal {
    pub subject_id: Uuid,
    pub actor_id: ActorId,
    pub name: String,
    pub principal_type: AuthenticatedPrincipalType,
    pub credential_id: Option<Uuid>,
    pub roles: Vec<String>,
}

impl ActorPrincipal {
    #[must_use]
    pub fn is_human(&self) -> bool {
        self.principal_type == AuthenticatedPrincipalType::User
    }

    #[must_use]
    pub fn is_administrator(&self) -> bool {
        self.is_human() && self.roles.iter().any(|role| role == "Admin")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActorType {
    User,
    System,
    Agent,
    ServiceAccount,
    Team,
}

impl ActorType {
    #[must_use]
    pub const fn as_database_str(self) -> &'static str {
        match self {
            Self::User => "User",
            Self::System => "System",
            Self::Agent => "Agent",
            Self::ServiceAccount => "ServiceAccount",
            Self::Team => "Team",
        }
    }

    #[must_use]
    pub fn from_database_str(value: &str) -> Option<Self> {
        match value {
            "User" => Some(Self::User),
            "System" => Some(Self::System),
            "Agent" => Some(Self::Agent),
            "ServiceAccount" => Some(Self::ServiceAccount),
            "Team" => Some(Self::Team),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuthenticatedPrincipalType {
    User,
    ServiceAccount,
    System,
    Agent,
}
