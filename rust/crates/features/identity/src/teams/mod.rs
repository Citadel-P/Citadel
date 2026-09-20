use crate::ActorType;
use crate::Clock;
use crate::EntitlementService;
use crate::IdentityError;
use crate::PagedResult;
use crate::PatchField;
use crate::ResourceInfo;
use crate::StoredPage;
use crate::permission_matrix;
use crate::validate_name;
use chrono::{DateTime, Utc};
use citadel_primitives::{ActorId, PermissionLevel, ResourceType, SpecificPermission};
use futures_util::future::BoxFuture;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::sync::Arc;
use uuid::Uuid;

mod read_models;
pub use read_models::{
    TeamDetails, TeamMemberDetails, TeamResourceAccessDetails, TeamSearchItemDetails,
};
mod commands;
pub use commands::{
    AddTeamMember, AddTeamRole, CreateTeam, DeleteTeams, NewTeamMutation, PatchTeam, RenameTeam,
    TeamPatchMutation, TeamResourceAccessInput,
};
mod repository;
pub use repository::{TeamReader, TeamRepository};
mod service;
pub use service::{TeamMutationService, TeamReadService};

pub mod model;
pub use model::Team;
