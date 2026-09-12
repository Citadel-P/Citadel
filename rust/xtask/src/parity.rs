//! Migration closure gate, separate from generated-artifact freshness.
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde_json::Value;

#[derive(Debug, PartialEq, Eq)]
struct Operation {
    method: String,
    path: String,
    public: bool,
}

pub fn check() -> Result<(), Box<dyn std::error::Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let full: Value =
        serde_json::from_slice(&std::fs::read(root.join("src/schema/Citadel.WebApi.json"))?)?;
    let public: Value = serde_json::from_slice(&std::fs::read(
        root.join("src/schema/Citadel.WebApi_public.json"),
    )?)?;
    let expected = reference(&full, &public)?;
    let mut actual = BTreeMap::new();
    let document = crate::openapi_gen::document(false);
    for (path, item) in document["paths"]
        .as_object()
        .ok_or("Generated API has no paths")?
    {
        for (method, operation) in item.as_object().ok_or("Invalid generated path item")? {
            let Some(id) = operation["operationId"].as_str() else {
                continue;
            };
            if actual
                .insert(
                    id.to_owned(),
                    Operation {
                        method: method.to_owned(),
                        path: path.to_owned(),
                        public: operation["x-citadel-public"].as_bool().unwrap_or(false),
                    },
                )
                .is_some()
            {
                return Err(format!("Duplicate Rust operation ID: {id}").into());
            }
        }
    }
    let gaps = compare(&expected, &actual);
    for gap in &gaps {
        eprintln!("{gap}");
    }
    if !gaps.is_empty() {
        return Err(format!("{} API parity gaps across {} .NET operations. This is a catalog check, not behavioral acceptance.", gaps.len(), expected.len()).into());
    }
    println!(
        "All {} .NET operation IDs, methods, paths and public exposure match. Behavioral and actual-router tests remain required.",
        expected.len()
    );
    Ok(())
}

fn reference(full: &Value, public: &Value) -> Result<BTreeMap<String, Operation>, String> {
    let mut operations = BTreeMap::new();
    let paths = full["paths"].as_object().ok_or("Reference has no paths")?;
    // Public .NET route groups can use different parameter names from the full
    // document (deploymentId versus id). Operation ID is their stable identity.
    let public_ids: BTreeSet<&str> = public["paths"]
        .as_object()
        .into_iter()
        .flat_map(|paths| paths.values())
        .filter_map(Value::as_object)
        .flat_map(|item| item.values())
        .filter_map(|operation| operation["operationId"].as_str())
        .collect();
    for (path, item) in paths {
        for method in [
            "get", "post", "put", "patch", "delete", "head", "options", "trace",
        ] {
            let Some(operation) = item.get(method) else {
                continue;
            };
            let id = operation["operationId"]
                .as_str()
                .ok_or_else(|| format!("Missing operation ID: {method} {path}"))?;
            let public = public_ids.contains(id);
            if operations
                .insert(
                    id.into(),
                    Operation {
                        method: method.into(),
                        path: path.clone(),
                        public,
                    },
                )
                .is_some()
            {
                return Err(format!("Duplicate reference operation ID: {id}"));
            }
        }
    }
    if operations.is_empty() {
        return Err("Reference contains no operations".into());
    }
    Ok(operations)
}

fn compare(
    expected: &BTreeMap<String, Operation>,
    actual: &BTreeMap<String, Operation>,
) -> Vec<String> {
    expected
        .iter()
        .filter_map(|(id, expected)| match actual.get(id) {
            None => Some(format!(
                "Missing {id}: {} {}",
                expected.method, expected.path
            )),
            Some(actual) if actual != expected => Some(format!(
                "Mismatch {id}: expected {expected:?}, found {actual:?}"
            )),
            _ => None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn missing_and_changed_operations_are_not_hidden_by_rust_only_routes() {
        let full = json!({"paths":{"/api/a":{"get":{"operationId":"a"}},"/api/b":{"post":{"operationId":"b"}}}});
        let expected = reference(&full, &full).unwrap();
        let mut actual = reference(&json!({"paths":{"/api/a":{"get":{"operationId":"a"}},"/health":{"get":{"operationId":"health"}}}}), &full).unwrap();
        assert_eq!(compare(&expected, &actual).len(), 1);
        actual.get_mut("a").unwrap().public = false;
        assert_eq!(compare(&expected, &actual).len(), 2);
        actual.get_mut("a").unwrap().public = true;
        actual.get_mut("a").unwrap().method = "post".into();
        assert_eq!(compare(&expected, &actual).len(), 2);
        assert!(compare(&expected, &expected).is_empty());
    }

    #[test]
    fn invalid_or_duplicate_reference_operations_fail_closed() {
        for document in [
            json!({}),
            json!({"paths":{}}),
            json!({"paths":{"/a":{"get":{}}}}),
            json!({"paths":{"/a":{"get":{"operationId":"same"}},"/b":{"get":{"operationId":"same"}}}}),
        ] {
            assert!(reference(&document, &json!({})).is_err());
        }
    }

    #[test]
    fn public_exposure_uses_operation_identity_not_path_parameter_spelling() {
        let full = json!({"paths":{"/a/{id}":{"get":{"operationId":"read"}}}});
        let public = json!({"paths":{"/a/{resourceId}":{"get":{"operationId":"read"}}}});
        assert!(reference(&full, &public).unwrap()["read"].public);
    }
}
