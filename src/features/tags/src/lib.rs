#![forbid(unsafe_code)]
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
mod model;
pub use model::{Tag, TagSummary, TaggableResourceType};
mod commands;
pub use commands::{NewTag, TagPatch};
mod validation;
pub use validation::normalize_color;

mod repository;
pub use repository::TagRepository;
mod error;
pub use error::TagError;
mod name_validation;
use name_validation::validate_name;
