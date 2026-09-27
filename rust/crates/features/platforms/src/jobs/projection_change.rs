//! Outcome returned only after a runtime projection transaction commits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectionChange {
    /// No usable identity/observation, or a snapshot superseded by a concurrent write.
    Unavailable,
    /// Accepted observation; freshness may advance without a semantic revision.
    Unchanged,
    Changed,
}
impl ProjectionChange {
    pub fn accepted(self) -> bool {
        self != Self::Unavailable
    }
    pub fn changed(self) -> bool {
        self == Self::Changed
    }
    pub fn committed(changed: bool) -> Self {
        if changed {
            Self::Changed
        } else {
            Self::Unchanged
        }
    }
}
