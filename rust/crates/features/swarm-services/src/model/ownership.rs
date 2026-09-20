use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SwarmServiceOwnership {
    #[default]
    Unmanaged,
    DockerStackExternal,
    CitadelService,
    CitadelStack,
    OwnershipConflict,
    System,
}
