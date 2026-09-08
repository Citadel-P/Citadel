use citadel_domain::ActorType;
use futures_util::future::BoxFuture;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::IdentityError;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActorView {
    pub id: Uuid,
    pub name: String,
    #[serde(rename = "type")]
    pub actor_type: ActorType,
    pub is_enabled: bool,
}

impl ActorView {
    pub fn set_enabled(&mut self, enabled: bool) -> Result<(), IdentityError> {
        if !matches!(
            self.actor_type,
            ActorType::User | ActorType::Team | ActorType::ServiceAccount
        ) {
            return Err(IdentityError::Validation(format!(
                "Actors of type {:?} cannot be enabled or disabled.",
                self.actor_type
            )));
        }
        self.is_enabled = enabled;
        Ok(())
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PatchActorEnabledInput {
    pub is_enabled: bool,
}

pub trait ActorStore: Send + Sync {
    fn get(&self, id: Uuid) -> BoxFuture<'_, Result<ActorView, IdentityError>>;
    fn set_enabled(
        &self,
        id: Uuid,
        enabled: bool,
    ) -> BoxFuture<'_, Result<ActorView, IdentityError>>;
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_user_team_and_service_account_actor_states_can_change() {
        for actor_type in [
            ActorType::User,
            ActorType::Team,
            ActorType::ServiceAccount,
            ActorType::Agent,
            ActorType::System,
        ] {
            let mut actor = ActorView {
                id: Uuid::now_v7(),
                name: "actor".into(),
                actor_type,
                is_enabled: true,
            };
            let mutable = matches!(
                actor_type,
                ActorType::User | ActorType::Team | ActorType::ServiceAccount
            );
            assert_eq!(actor.set_enabled(false).is_ok(), mutable);
            assert_eq!(actor.is_enabled, !mutable);
        }
    }
}
