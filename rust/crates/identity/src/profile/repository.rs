use super::*;

pub trait ProfileRepository: Send + Sync {
    fn get_current(
        &self,
        user_id: Uuid,
    ) -> BoxFuture<'_, Result<Option<CurrentProfileRecord>, IdentityError>>;

    fn get_user(&self, user_id: Uuid) -> BoxFuture<'_, Result<Option<User>, IdentityError>>;

    fn rename_user<'a>(
        &'a self,
        user_id: Uuid,
        new_name: &'a str,
        actor_id: citadel_primitives::ActorId,
        renamed_at: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<CurrentProfileRecord, IdentityError>>;

    fn get_preferences(
        &self,
        user_id: Uuid,
    ) -> BoxFuture<'_, Result<Option<UserPreferences>, IdentityError>>;

    fn patch_preferences<'a>(
        &'a self,
        user_id: Uuid,
        update: &'a UserPreferencesUpdate,
        updated_at: DateTime<Utc>,
        actor_id: citadel_primitives::ActorId,
    ) -> BoxFuture<'a, Result<UserPreferences, IdentityError>>;

    fn change_password<'a>(
        &'a self,
        user_id: Uuid,
        expected_password_hash: &'a str,
        new_password_hash: &'a str,
        current_session_id: Option<Uuid>,
        changed_at: DateTime<Utc>,
        actor_id: citadel_primitives::ActorId,
    ) -> BoxFuture<'a, Result<PasswordChangeOutcome, IdentityError>>;
}
