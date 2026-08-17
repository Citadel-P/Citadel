#![forbid(unsafe_code)]

mod activity;
mod enums;
mod license;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub use activity::*;
pub use enums::*;
pub use license::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ActorId(Uuid);

impl ActorId {
    #[must_use]
    pub const fn new(value: Uuid) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn value(self) -> Uuid {
        self.0
    }
}
