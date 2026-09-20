use crate::ActorType;
use crate::{
    Clock, EntitlementService, IdentityError, PagedResult, PatchField, ResourceInfo, StoredPage,
    permission_matrix, validate_name,
};
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
