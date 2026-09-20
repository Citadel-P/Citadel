use super::*;

impl ActorDetails {
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
            let mut actor = ActorDetails {
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
