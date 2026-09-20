use std::collections::BTreeMap;

use crate::{ComposeModel, ResolvedStackBindings};

const MAX_LISTED_ENTRIES: usize = 10;
const MAX_VALUE_CHARACTERS: usize = 80;

pub(super) fn compose_resolution_message(
    model: &ComposeModel,
    resolved: &ResolvedStackBindings,
    source_env_file_count: usize,
    redactions: &[String],
) -> String {
    // Use the same effective, case-insensitive lookup as environment resolution.
    // Optional references without a binding and unused bindings are not resolved.
    let bindings = resolved
        .entries
        .iter()
        .map(|entry| (entry.name.to_ascii_lowercase(), entry))
        .collect::<BTreeMap<_, _>>();
    let used = model
        .variables
        .iter()
        .filter_map(|name| {
            bindings
                .get(&name.to_ascii_lowercase())
                .map(|entry| (name, *entry))
        })
        .collect::<Vec<_>>();
    let mut groups = Vec::new();
    for (secret, singular) in [(false, "variable"), (true, "secret")] {
        let entries = used
            .iter()
            .filter(|(_, entry)| entry.secret == secret)
            .collect::<Vec<_>>();
        if entries.is_empty() {
            continue;
        }
        let names = entries
            .iter()
            .take(MAX_LISTED_ENTRIES)
            .map(|(name, entry)| {
                if secret {
                    (*name).clone()
                } else {
                    // Redact before truncating: never expose a prefix of a secret
                    // embedded in a variable when its full value is longer than 80 chars.
                    let value = super::redact(entry.value.to_string(), redactions);
                    let value = value
                        .replace('\r', "\\r")
                        .replace('\n', "\\n")
                        .replace('\t', "\\t");
                    let value = if value.chars().count() > MAX_VALUE_CHARACTERS {
                        format!(
                            "{}...",
                            value
                                .chars()
                                .take(MAX_VALUE_CHARACTERS - 3)
                                .collect::<String>()
                        )
                    } else {
                        value
                    };
                    format!("{name}={value}")
                }
            })
            .collect::<Vec<_>>()
            .join(", ");
        let remaining = entries.len().saturating_sub(MAX_LISTED_ENTRIES);
        let suffix = if remaining > 0 {
            format!(", ... (+{remaining} more)")
        } else {
            String::new()
        };
        groups.push(format!(
            "{} {names}{suffix}",
            count(entries.len(), singular)
        ));
    }
    let mut message = if groups.is_empty() {
        "No Citadel variables or secrets were referenced by the compose files.".to_owned()
    } else {
        format!(
            "Resolved {} for compose interpolation.",
            groups.join(" and ")
        )
    };
    if source_env_file_count > 0 {
        message.push_str(&format!(
            " Included {}.",
            count(source_env_file_count, "repo env file")
        ));
    }
    message
}

fn count(value: usize, singular: &str) -> String {
    format!("{value} {singular}{}", if value == 1 { "" } else { "s" })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ResourceBindingSnapshot, StackBinding, parse_compose};

    fn binding(name: &str, value: &str, secret: bool) -> StackBinding {
        StackBinding {
            name: name.into(),
            value: value.to_owned().into(),
            secret,
            snapshot: ResourceBindingSnapshot {
                name: name.into(),
                kind: if secret { "Secret" } else { "Variable" }.into(),
                scope: "Stack".into(),
                value: if secret { "********" } else { value }.into(),
                secret_id: None,
                secret_delivery_mode: None,
                target_path: None,
            },
        }
    }

    fn message(bindings: Vec<StackBinding>, names: &[&str], env_files: usize) -> String {
        let mut model = parse_compose(&["services:\n  web:\n    image: nginx\n".into()]).unwrap();
        model.variables = names.iter().map(|name| (*name).to_owned()).collect();
        let redactions = bindings
            .iter()
            .filter(|entry| entry.secret)
            .map(|entry| entry.value.to_string())
            .collect::<Vec<_>>();
        compose_resolution_message(
            &model,
            &ResolvedStackBindings { entries: bindings },
            env_files,
            &redactions,
        )
    }

    #[test]
    fn matches_dotnet_compose_interpolation_summary() {
        assert_eq!(
            message(
                vec![
                    binding("API_KEY", "never-log-this", true),
                    binding("APP_MODE", "prod", false)
                ],
                &["APP_MODE", "API_KEY"],
                1
            ),
            "Resolved 1 variable APP_MODE=prod and 1 secret API_KEY for compose interpolation. Included 1 repo env file."
        );
    }

    #[test]
    fn counts_only_effective_referenced_bindings_and_pluralizes() {
        assert_eq!(
            message(
                vec![
                    binding("UNUSED", "ignored", false),
                    binding("port", "80", false),
                    binding("PORT", "8090", false),
                    binding("MODE", "prod", false)
                ],
                &["PORT", "MODE", "OPTIONAL"],
                2
            ),
            "Resolved 2 variables MODE=prod, PORT=8090 for compose interpolation. Included 2 repo env files."
        );
        assert_eq!(
            message(
                vec![
                    binding("B", "secret-b", true),
                    binding("A", "secret-a", true)
                ],
                &["A", "B"],
                0
            ),
            "Resolved 2 secrets A, B for compose interpolation."
        );
    }

    #[test]
    fn empty_resolution_and_repository_files_are_explicit() {
        assert_eq!(
            message(vec![], &[], 0),
            "No Citadel variables or secrets were referenced by the compose files."
        );
        assert_eq!(
            message(vec![binding("UNUSED", "x", false)], &["OPTIONAL"], 1),
            "No Citadel variables or secrets were referenced by the compose files. Included 1 repo env file."
        );
    }

    #[test]
    fn values_are_single_line_bounded_and_redacted_before_truncation() {
        let secret = "sensitive".repeat(20);
        let result = message(
            vec![
                binding("KEY", &secret, true),
                binding("VALUE", &format!("line\r\n\t{secret}"), false),
            ],
            &["KEY", "VALUE"],
            0,
        );
        assert!(result.contains("VALUE=line\\r\\n\\t********"));
        assert!(!result.contains("sensitive"));
        let result = message(
            vec![binding("VALUE", &"é".repeat(100), false)],
            &["VALUE"],
            0,
        );
        assert!(result.contains(&format!("VALUE={}...", "é".repeat(77))));
    }

    #[test]
    fn long_lists_keep_accurate_counts_without_listing_every_value() {
        let names = (0..25)
            .map(|index| format!("VAR_{index:02}"))
            .collect::<Vec<_>>();
        let result = message(
            names
                .iter()
                .map(|name| binding(name, "value", false))
                .collect(),
            &names.iter().map(String::as_str).collect::<Vec<_>>(),
            0,
        );
        assert!(result.starts_with("Resolved 25 variables VAR_00=value"));
        assert!(result.contains("... (+15 more)"));
        assert!(!result.contains("VAR_10="));
    }
}
