use crate::ActorType;
use crate::Clock;
use crate::EntitlementService;
use crate::IdentityError;
use crate::PagedResult;
use crate::PatchField;
use crate::ResourceInfo;
use crate::StoredPage;
use crate::validate_name;
use crate::{ResourceAccessDetails, ResourceAccessInput};
use chrono::{DateTime, Utc};
use citadel_primitives::ActorId;
#[cfg(test)]
use citadel_primitives::{PermissionLevel, ResourceType, SpecificPermission};
use futures_util::future::BoxFuture;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

mod read_models;
pub use read_models::{TeamDetails, TeamMemberDetails, TeamSearchItemDetails};
mod commands;
pub use commands::{
    AddTeamMember, AddTeamRole, CreateTeam, DeleteTeams, NewTeamMutation, PatchTeam, RenameTeam,
    TeamPatchMutation,
};
mod repository;
pub use repository::{TeamReader, TeamRepository};
mod service;
pub use service::{TeamMutationService, TeamReadService};

pub mod model;
pub use model::Team;
