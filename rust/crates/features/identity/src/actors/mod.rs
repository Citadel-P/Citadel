use crate::IdentityError;

use futures_util::future::BoxFuture;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

mod read_models;
pub use read_models::ActorDetails;
mod commands;
pub use commands::PatchActorEnabledInput;
mod repository;
pub use repository::ActorRepository;
mod service;

pub mod model;
pub use model::{ActorPrincipal, ActorType, AuthenticatedPrincipalType, SYSTEM_ACTOR_ID};
