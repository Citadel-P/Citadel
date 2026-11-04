//! Safe summaries for the existing instance-scoped alert observations.
use crate::{AgentUpdateEligibility, AgentUpdateRelease, AgentUpdateTarget};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

/// Unknown observations cannot prove a previously advertised installation is current.
/// IDs are internal correlation keys; only names and versions enter the human message.
pub fn agent_availability(
    release: &AgentUpdateRelease,
    targets: &[AgentUpdateTarget],
    previous: &Value,
) -> (bool, Value) {
    let mut available = BTreeMap::new();
    let mut uncertain = BTreeSet::new();
    let mut current = BTreeSet::new();
    for target in targets {
        match target.eligibility(Some(release)) {
            AgentUpdateEligibility::Available => {
                available.entry(&target.id).or_insert_with(|| {
                    json!({
                        "Id": target.id, "Name": target.name, "Version": target.installed_version,
                    })
                });
            }
            AgentUpdateEligibility::Current | AgentUpdateEligibility::Incompatible => {
                current.insert(&target.id);
            }
            _ => {
                uncertain.insert(&target.id);
            }
        }
    }
    let pending: Vec<_> = if previous["TargetVersion"] == release.version {
        ["Agents", "PendingAgents"]
            .into_iter()
            .flat_map(|key| previous[key].as_array().into_iter().flatten())
            .filter(|agent| {
                agent["Id"].as_str().is_some_and(|id| {
                    uncertain.iter().any(|candidate| candidate.as_str() == id)
                        && !current.iter().any(|candidate| candidate.as_str() == id)
                        && !available.keys().any(|candidate| candidate.as_str() == id)
                })
            })
            .cloned()
            .collect()
    } else {
        vec![]
    };
    let agents: Vec<_> = available.into_values().collect();
    let count = agents.len();
    let names = agents
        .iter()
        .map(|agent| {
            format!(
                "{} ({})",
                agent["Name"].as_str().unwrap_or_default(),
                agent["Version"].as_str().unwrap_or_default()
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    let message = if count > 0 {
        format!(
            "{count} Agent installation(s) can update to {}: {names}.",
            release.version
        )
    } else {
        format!(
            "Awaiting confirmation of previously advertised Agent updates to {}.",
            release.version
        )
    };
    (
        !agents.is_empty() || !pending.is_empty(),
        json!({
            "TargetVersion": release.version, "Count": count, "Agents": agents,
            "PendingAgents": pending, "HumanMessage": message,
        "CompleteInventory": uncertain.is_empty(),
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn agent(
        id: &str,
        version: Option<&str>,
        connected: bool,
        supported: bool,
    ) -> AgentUpdateTarget {
        AgentUpdateTarget {
            id: id.into(),
            name: id.into(),
            installed_version: version.map(Into::into),
            connected,
            supported,
            resource_id: uuid::Uuid::now_v7(),
            kind: crate::AgentUpdateKind::Platform,
            reason: None,
            nodes: 1,
            drain_nodes: vec![],
            revision: String::new(),
        }
    }
    #[test]
    fn availability_counts_installations_and_excludes_ineligible_observations() {
        let release = AgentUpdateRelease {
            version: "0.1.2".into(),
            image: "verified".into(),
        };
        let targets = vec![
            agent("a", Some("0.1.1"), true, true),
            agent("a", Some("0.1.1"), true, true),
            agent("b", Some("0.1.2"), true, true),
            agent("c", Some("0.1.2"), true, true),
            agent("offline", Some("0.1.1"), false, true),
            agent("unknown", None, true, true),
            agent("manual", Some("0.1.1"), true, false),
        ];
        let (matched, info) = agent_availability(&release, &targets, &Value::Null);
        assert!(matched);
        assert_eq!(info["Count"], 1);
        assert_eq!(info["Agents"][0]["Name"], "a");
        assert_eq!(agent_availability(&release, &targets, &info).1, info);
    }
    #[test]
    fn partial_updates_unknown_observations_and_removed_registrations_settle_safely() {
        let release = AgentUpdateRelease {
            version: "0.1.2".into(),
            image: "verified".into(),
        };
        let targets = vec![
            agent("a", Some("0.1.1"), true, true),
            agent("b", Some("0.1.1"), true, true),
        ];
        let (_, prior) = agent_availability(&release, &targets, &Value::Null);
        let partial = vec![
            agent("a", Some("0.1.2"), true, true),
            agent("b", Some("0.1.1"), true, true),
        ];
        let (matched, summary) = agent_availability(&release, &partial, &prior);
        assert!(matched);
        assert_eq!(summary["Count"], 1);
        for target in [
            agent("b", None, true, true),
            agent("b", Some("0.1.1"), false, true),
            agent("b", Some("0.1.1"), true, false),
        ] {
            let (matched, pending) = agent_availability(&release, &[target], &summary);
            assert!(matched);
            assert_eq!(pending["Count"], 0);
            assert_eq!(pending["PendingAgents"].as_array().unwrap().len(), 1);
        }
        assert!(
            !agent_availability(&release, &[agent("b", Some("0.1.2"), true, true)], &summary).0
        );
        assert!(!agent_availability(&release, &[], &summary).0);
        let newer = AgentUpdateRelease {
            version: "0.1.3".into(),
            ..release
        };
        assert!(!agent_availability(&newer, &[agent("b", None, false, true)], &summary).0);
    }
}
