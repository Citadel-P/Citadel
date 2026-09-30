citadel_primitives::status_enum! {
    pub enum GitRepositoryStatus {
        Unknown,
        Pending,
        Created,
        Healthy,
        Degraded,
    }
}

citadel_primitives::status_enum! {
    pub enum GitRepositoryRefStatus {
        Pending,
        Syncing,
        Healthy,
        Degraded,
    }
}

impl From<GitRepositoryRefStatus> for GitRepositoryStatus {
    fn from(status: GitRepositoryRefStatus) -> Self {
        match status {
            GitRepositoryRefStatus::Pending | GitRepositoryRefStatus::Syncing => Self::Pending,
            GitRepositoryRefStatus::Healthy => Self::Healthy,
            GitRepositoryRefStatus::Degraded => Self::Degraded,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reference_states_map_to_valid_repository_states() {
        for (reference, expected) in [
            (
                GitRepositoryRefStatus::Pending,
                GitRepositoryStatus::Pending,
            ),
            (
                GitRepositoryRefStatus::Syncing,
                GitRepositoryStatus::Pending,
            ),
            (
                GitRepositoryRefStatus::Healthy,
                GitRepositoryStatus::Healthy,
            ),
            (
                GitRepositoryRefStatus::Degraded,
                GitRepositoryStatus::Degraded,
            ),
        ] {
            assert_eq!(GitRepositoryStatus::from(reference), expected);
        }
        assert!("Syncing".parse::<GitRepositoryStatus>().is_err());
        assert!("Failed".parse::<GitRepositoryRefStatus>().is_err());
    }
}
