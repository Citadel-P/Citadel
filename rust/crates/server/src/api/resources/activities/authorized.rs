use citadel_identity::ActorPrincipal;
pub(crate) fn activity_access(principal: &ActorPrincipal) -> citadel_activities::ActivityAccess {
    citadel_activities::ActivityAccess {
        actor_id: principal.actor_id,
        administrator: principal.is_administrator(),
    }
}
