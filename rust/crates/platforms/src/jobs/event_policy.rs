/// Returns whether a Docker or Agent daemon event can change a persisted
/// Platform inventory projection.
#[must_use]
pub fn triggers_inventory_reconciliation(resource_type: &str, action: &str) -> bool {
    matches!(
        resource_type.to_ascii_lowercase().as_str(),
        "container"
            | "image"
            | "network"
            | "volume"
            | "node"
            | "service"
            | "task"
            | "secret"
            | "config"
            | "builder"
    ) && !action.is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_inventory_events_trigger_reconciliation() {
        for resource_type in [
            "container",
            "image",
            "network",
            "volume",
            "node",
            "service",
            "task",
            "secret",
            "config",
            "builder",
        ] {
            assert!(triggers_inventory_reconciliation(resource_type, "update"));
        }
        assert!(!triggers_inventory_reconciliation("plugin", "enable"));
        assert!(!triggers_inventory_reconciliation("service", ""));
    }
}
