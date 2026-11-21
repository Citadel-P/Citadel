//! Outcome returned only after a runtime projection transaction commits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectionChange {
    /// No usable identity/observation, or a snapshot superseded by a concurrent write.
    Unavailable,
    /// Accepted observation; freshness may advance without a semantic revision.
    Unchanged,
    Changed,
    /// Stack observed in the committed container mutation, including a deleted row.
    ChangedWithStack(uuid::Uuid),
}
impl ProjectionChange {
    pub fn accepted(self) -> bool {
        self != Self::Unavailable
    }
    pub fn changed(self) -> bool {
        matches!(self, Self::Changed | Self::ChangedWithStack(_))
    }
    pub fn affected_stack(self) -> Option<uuid::Uuid> {
        match self {
            Self::ChangedWithStack(id) => Some(id),
            _ => None,
        }
    }
    pub fn committed(changed: bool) -> Self {
        if changed {
            Self::Changed
        } else {
            Self::Unchanged
        }
    }
}
