use chrono::{DateTime, Utc};
use citadel_primitives::ActorId;
use uuid::Uuid;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct User {
    id: Uuid,
    name: String,
    email: String,
    password_hash: Option<String>,
    actor_id: ActorId,
    created_by_actor_id: ActorId,
    created_at: DateTime<Utc>,
}

impl User {
    #[must_use]
    pub fn new(
        name: String,
        email: String,
        password_hash: Option<String>,
        actor_id: ActorId,
        created_by_actor_id: ActorId,
        created_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id: Uuid::now_v7(),
            name,
            email,
            password_hash,
            actor_id,
            created_by_actor_id,
            created_at,
        }
    }

    #[must_use]
    pub fn from_persistence(
        id: Uuid,
        name: String,
        email: String,
        password_hash: Option<String>,
        actor_id: ActorId,
        created_by_actor_id: ActorId,
        created_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            name,
            email,
            password_hash,
            actor_id,
            created_by_actor_id,
            created_at,
        }
    }

    #[must_use]
    pub const fn id(&self) -> Uuid {
        self.id
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn email(&self) -> &str {
        &self.email
    }

    #[must_use]
    pub fn password_hash(&self) -> Option<&str> {
        self.password_hash.as_deref()
    }

    #[must_use]
    pub const fn actor_id(&self) -> ActorId {
        self.actor_id
    }

    #[must_use]
    pub const fn created_by_actor_id(&self) -> ActorId {
        self.created_by_actor_id
    }

    #[must_use]
    pub const fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    pub fn rename(&mut self, name: String) {
        self.name = name;
    }

    pub fn set_password_hash(&mut self, password_hash: String) {
        self.password_hash = Some(password_hash);
    }
}
