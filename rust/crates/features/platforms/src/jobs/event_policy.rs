/// Returns whether a Docker or Agent daemon event can change a persisted
/// Platform inventory projection.
#[must_use]
pub fn triggers_inventory_reconciliation(resource_type: &str, _action: &str) -> bool {
    // Unknown/new resource kinds and incomplete payloads must converge through
    // a full refresh. Exclude only events known not to change this projection.
    !matches!(
        resource_type.to_ascii_lowercase().as_str(),
        "builder" | "plugin"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inventory_and_unknown_events_trigger_reconciliation() {
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
        ] {
            assert!(triggers_inventory_reconciliation(resource_type, "update"));
        }
        assert!(!triggers_inventory_reconciliation("builder", "prune"));
        assert!(!triggers_inventory_reconciliation("plugin", "enable"));
        assert!(triggers_inventory_reconciliation("service", ""));
        assert!(triggers_inventory_reconciliation(
            "future-resource",
            "change"
        ));
        assert!(triggers_inventory_reconciliation("", ""));
    }
}
