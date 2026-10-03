citadel_primitives::status_enum! {
    pub enum SwarmServiceHealth {
        Unknown,
        Healthy,
        Progressing,
        Degraded,
        Failed,
        Created,
        Stopped,
    }
}

citadel_primitives::status_enum! {
    pub enum SwarmServiceSynchronizationState {
        NeverApplied,
        DesiredChangesPending,
        InSync,
        Drifted,
        RuntimeMissing,
        OutcomeUnknown,
        OwnershipConflict,
    }
}

citadel_primitives::status_enum! {
    pub enum SwarmServiceOperationKind {
        Apply,
        Scale,
        ForceUpdate,
        Delete,
    }
}

citadel_primitives::status_enum! {
    pub enum SwarmServiceOperationState {
        Prepared,
        Canceled,
        PendingAcceptance,
        Accepted,
        Rejected,
        NotAccepted,
        OutcomeUnknown,
        Completed,
        OwnershipConflict,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn operation_states_preserve_persisted_spelling_and_reject_unknown_values() {
        for state in SwarmServiceOperationState::ALL {
            assert_eq!(
                state
                    .as_str()
                    .parse::<SwarmServiceOperationState>()
                    .unwrap(),
                *state
            );
            assert_eq!(serde_json::to_value(state).unwrap(), state.as_str());
        }
        assert_eq!(
            "Canceled".parse::<SwarmServiceOperationState>().unwrap(),
            SwarmServiceOperationState::Canceled
        );
        for invalid in ["Cancelled", "Running", "completed", "Invalid"] {
            assert!(invalid.parse::<SwarmServiceOperationState>().is_err());
        }
    }

    #[test]
    fn health_and_synchronization_are_distinct_vocabularies() {
        for health in SwarmServiceHealth::ALL {
            assert_eq!(
                health.as_str().parse::<SwarmServiceHealth>().unwrap(),
                *health
            );
        }
        for state in SwarmServiceSynchronizationState::ALL {
            assert_eq!(
                state
                    .as_str()
                    .parse::<SwarmServiceSynchronizationState>()
                    .unwrap(),
                *state
            );
        }
        assert!(
            "Healthy"
                .parse::<SwarmServiceSynchronizationState>()
                .is_err()
        );
        assert!("InSync".parse::<SwarmServiceHealth>().is_err());
        assert_eq!(
            "Delete".parse::<SwarmServiceOperationKind>().unwrap(),
            SwarmServiceOperationKind::Delete
        );
    }
}
