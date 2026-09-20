use citadel_primitives::ActorId;
use uuid::Uuid;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Team {
    id: Uuid,
    name: String,
    actor_id: ActorId,
}

impl Team {
    #[must_use]
    pub fn new(name: String, actor_id: ActorId) -> Self {
        Self {
            id: Uuid::now_v7(),
            name,
            actor_id,
        }
    }

    #[must_use]
    pub const fn from_persistence(id: Uuid, name: String, actor_id: ActorId) -> Self {
        Self { id, name, actor_id }
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
    pub const fn actor_id(&self) -> ActorId {
        self.actor_id
    }

    pub fn rename(&mut self, name: String) {
        self.name = name;
    }
}
