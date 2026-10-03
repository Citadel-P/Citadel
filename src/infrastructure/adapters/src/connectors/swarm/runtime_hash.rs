//! Wire-compatible with Hosting.Common.SwarmServiceRuntimeStateHasher. Hash only
//! represented configuration, normalizing Docker defaults and excluding ownership.
use serde_json::Value;
use sha2::{Digest, Sha256};

pub(crate) fn legacy_hash(spec: &Value) -> String {
    Sha256::digest(serde_json::to_vec(spec).expect("Docker JSON serializes"))
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

pub(crate) fn hash(spec: &Value) -> String {
    let task = &spec["TaskTemplate"];
    let container = &task["ContainerSpec"];
    let health = &container["Healthcheck"];
    let resources = &task["Resources"];
    let restart = &task["RestartPolicy"];
    let update = &spec["UpdateConfig"];
    let mut value = String::with_capacity(1024);
    append(&mut value, &text(&container["Image"]));
    append(
        &mut value,
        if spec["Mode"].get("Global").is_some() {
            "Global"
        } else {
            "Replicated"
        },
    );
    append(&mut value, &text(&spec["Mode"]["Replicated"]["Replicas"]));
    sequence(&mut value, strings(&container["Command"]), true);
    sequence(&mut value, strings(&container["Args"]), true);
    sequence(&mut value, strings(&container["Env"]), false);
    append(&mut value, &text(&container["User"]));
    append(&mut value, &text(&container["Dir"]));
    sequence(&mut value, strings(&health["Test"]), true);
    for key in ["Interval", "Timeout", "Retries", "StartPeriod"] {
        append(&mut value, &text(&health[key]));
    }
    append(
        &mut value,
        &default_text(&container["StopGracePeriod"], 10_000_000_000),
    );
    sequence(
        &mut value,
        items(&spec["EndpointSpec"]["Ports"])
            .map(|p| {
                format!(
                    "{}/{}/{}/{}",
                    text(&p["TargetPort"]),
                    text(&p["PublishedPort"]),
                    text(&p["Protocol"]).to_lowercase(),
                    text(&p["PublishMode"]).to_lowercase()
                )
            })
            .collect(),
        false,
    );
    sequence(
        &mut value,
        items(
            task.get("Networks")
                .filter(|v| !v.is_null())
                .unwrap_or(&spec["Networks"]),
        )
        .map(|n| text(&n["Target"]))
        .collect(),
        false,
    );
    sequence(
        &mut value,
        items(&container["Mounts"])
            .map(|m| {
                format!(
                    "{}/{}/{}/{}",
                    text(&m["Type"]).to_lowercase(),
                    text(&m["Source"]),
                    text(&m["Target"]),
                    if m["ReadOnly"].as_bool().unwrap_or(false) {
                        "True"
                    } else {
                        "False"
                    }
                )
            })
            .collect(),
        false,
    );
    for (key, id, name) in [
        ("Secrets", "SecretID", "SecretName"),
        ("Configs", "ConfigID", "ConfigName"),
    ] {
        sequence(
            &mut value,
            items(&container[key])
                .map(|r| {
                    format!(
                        "{}/{}/{}",
                        text(&r[id]),
                        text(&r[name]),
                        text(&r["File"]["Name"])
                    )
                })
                .collect(),
            false,
        );
    }
    for group in ["Limits", "Reservations"] {
        for key in ["NanoCPUs", "MemoryBytes"] {
            append(&mut value, &text(&resources[group][key]));
        }
    }
    sequence(
        &mut value,
        strings(&task["Placement"]["Constraints"]),
        false,
    );
    let condition = text(&restart["Condition"]).to_lowercase().replace('-', "");
    append(&mut value, if condition == "any" { "" } else { &condition });
    append(&mut value, &default_text(&restart["Delay"], 5_000_000_000));
    for key in ["MaxAttempts", "Window"] {
        append(&mut value, &default_text(&restart[key], 0));
    }
    for key in ["Parallelism", "Delay"] {
        append(&mut value, &text(&update[key]));
    }
    for key in ["Order", "FailureAction"] {
        append(
            &mut value,
            &text(&update[key]).to_lowercase().replace('-', ""),
        );
    }
    let mut labels: Vec<_> = spec["Labels"]
        .as_object()
        .into_iter()
        .flatten()
        .filter(|(key, _)| !key.to_ascii_lowercase().starts_with("com.citadel."))
        .collect();
    labels.sort_by(|(a, _), (b, _)| a.encode_utf16().cmp(b.encode_utf16()));
    if !labels.is_empty() {
        for (key, label) in labels {
            append(&mut value, key);
            append(&mut value, &text(label));
        }
        value.push('|');
    }
    Sha256::digest(value.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
fn items(value: &Value) -> impl Iterator<Item = &Value> {
    value.as_array().into_iter().flatten()
}
fn strings(value: &Value) -> Vec<String> {
    items(value).map(text).collect()
}
fn text(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        _ => String::new(),
    }
}
fn default_text(value: &Value, default: i64) -> String {
    if value.as_i64() == Some(default) {
        String::new()
    } else {
        text(value)
    }
}
fn append(out: &mut String, value: &str) {
    use std::fmt::Write;
    write!(out, "{}:{value};", value.encode_utf16().count()).expect("write to String");
}
fn sequence(out: &mut String, mut values: Vec<String>, ordered: bool) {
    if !ordered {
        values.sort_by(|a, b| a.encode_utf16().cmp(b.encode_utf16()));
    }
    for value in values {
        append(out, &value);
    }
    out.push('|');
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn docker_defaults_ownership_and_force_update_do_not_change_configuration_hash() {
        let desired = json!({"Mode":{"Replicated":{"Replicas":1}},"TaskTemplate":{"ContainerSpec":{"Image":"nginx:latest","Env":["B=2","A=1"]}}});
        let mut observed = desired.clone();
        observed["Labels"] = json!({"com.citadel.operation-id":"new","com.citadel.managed":"true"});
        observed["TaskTemplate"]["ForceUpdate"] = json!(42);
        observed["TaskTemplate"]["RestartPolicy"] =
            json!({"Condition":"any","Delay":5000000000i64,"MaxAttempts":0,"Window":0});
        observed["TaskTemplate"]["ContainerSpec"]["StopGracePeriod"] = json!(10000000000i64);
        observed["TaskTemplate"]["ContainerSpec"]["Env"] = json!(["A=1", "B=2"]);
        assert_eq!(hash(&desired), hash(&observed));
        observed["TaskTemplate"]["ContainerSpec"]["Image"] = json!("nginx:other");
        assert_ne!(hash(&desired), hash(&observed));
    }
    #[test]
    fn command_order_and_configured_labels_are_significant() {
        let mut spec = json!({"TaskTemplate":{"ContainerSpec":{"Command":["sh","-c"]}}});
        let original = hash(&spec);
        spec["TaskTemplate"]["ContainerSpec"]["Command"] = json!(["-c", "sh"]);
        assert_ne!(original, hash(&spec));
        spec["TaskTemplate"]["ContainerSpec"]["Command"] = json!(["sh", "-c"]);
        spec["Labels"] = json!({"owner":"team"});
        assert_ne!(original, hash(&spec));
    }
}
