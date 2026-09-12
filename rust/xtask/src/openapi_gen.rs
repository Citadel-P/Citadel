//! Export Utoipa's handler-derived documents and check the frontend route surface.
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

pub fn generate(check: bool) -> Result<(), Box<dyn std::error::Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let full = document(false);
    let public = document(true);
    validate_references(&full)?;
    validate_references(&public)?;
    for (name, doc) in [("v1.json", &full), ("public-v1.json", &public)] {
        write_or_check(
            &root.parent().unwrap().join("schema").join(name),
            format!("{}\n", serde_json::to_string_pretty(doc)?).as_bytes(),
            check,
        )?;
    }
    write_or_check(
        &root.join("generated/frontend/foundation-api.ts"),
        frontend_types(&full)?.as_bytes(),
        check,
    )?;
    verify_frontend_contract(root, &full)?;
    println!(
        "{} {} full and {} public Utoipa operations",
        if check { "verified" } else { "generated" },
        operation_index(&full)?.len(),
        operation_index(&public)?.len()
    );
    Ok(())
}

pub(crate) fn document(public_only: bool) -> Value {
    serde_json::to_value(citadel_server::openapi::document(public_only))
        .expect("OpenAPI serializes")
}

struct Operation<'a> {
    method: &'a str,
    path: &'a str,
    value: &'a Value,
}

fn operation_index(
    document: &Value,
) -> Result<BTreeMap<&str, Operation<'_>>, Box<dyn std::error::Error>> {
    let mut operations = BTreeMap::new();
    for (path, item) in document["paths"]
        .as_object()
        .ok_or("OpenAPI has no paths")?
    {
        for (method, value) in item.as_object().ok_or("Invalid path item")? {
            let Some(id) = value["operationId"].as_str() else {
                continue;
            };
            if operations
                .insert(
                    id,
                    Operation {
                        method,
                        path,
                        value,
                    },
                )
                .is_some()
            {
                return Err(format!("Duplicate operation ID: {id}").into());
            }
        }
    }
    Ok(operations)
}

// This gate covers client operation identity, paths, parameters and declared errors.
// DTO schemas now come from Rust; field behavior is covered by serialization and
// HTTP tests instead of the former comparison that only compared $ref names.
fn verify_frontend_contract(root: &Path, full: &Value) -> Result<(), Box<dyn std::error::Error>> {
    let frontend: Value = serde_json::from_slice(&fs::read(
        root.parent()
            .unwrap()
            .join("src/Citadel.FrontEnd/src/api/schema/swagger.json"),
    )?)?;
    let expected = operation_index(&frontend)?;
    for (id, actual) in operation_index(full)? {
        if !actual.path.starts_with("/api/v1/") {
            continue;
        }
        let reference = expected
            .get(id)
            .ok_or_else(|| format!("Frontend is missing operation {id}"))?;
        if (actual.method, actual.path) != (reference.method, reference.path) {
            return Err(format!("Frontend route mismatch for {id}").into());
        }
        if parameters(actual.value) != parameters(reference.value) {
            return Err(format!(
                "Frontend parameters mismatch for {id}: {:?} versus {:?}",
                parameters(actual.value),
                parameters(reference.value)
            )
            .into());
        }
        if actual.value["tags"] != reference.value["tags"] {
            return Err(format!("Frontend tags mismatch for {id}").into());
        }
        if actual.method == "patch" && content_types(actual.value) != content_types(reference.value)
        {
            return Err(format!("Frontend PATCH media types mismatch for {id}").into());
        }
        verify_operation_metadata(id, actual.value, reference.value)?;
        if !error_statuses(reference.value).is_subset(&error_statuses(actual.value)) {
            return Err(format!("Frontend error responses mismatch for {id}").into());
        }
    }
    Ok(())
}

// Compare names rather than example payloads: Rust examples must use the Rust wire contract.
fn verify_operation_metadata(
    id: &str,
    actual: &Value,
    reference: &Value,
) -> Result<(), Box<dyn std::error::Error>> {
    for (media, content) in reference["requestBody"]["content"]
        .as_object()
        .into_iter()
        .flatten()
    {
        for name in content["examples"]
            .as_object()
            .into_iter()
            .flatten()
            .map(|(name, _)| name)
        {
            if actual["requestBody"]["content"][media]["examples"]
                .get(name)
                .is_none()
            {
                return Err(format!("Missing {media} request example {name} for {id}").into());
            }
        }
    }
    for (status, response) in reference["responses"].as_object().into_iter().flatten() {
        for name in response["headers"]
            .as_object()
            .into_iter()
            .flatten()
            .map(|(name, _)| name)
        {
            // .NET emitted cookie names as headers, sometimes on a synthetic 200.
            // Rust documents real Set-Cookie headers on the actual response status.
            let preserved = if name == "refresh_token" {
                actual["responses"]
                    .as_object()
                    .into_iter()
                    .flatten()
                    .any(|(_, response)| {
                        response["headers"]["Set-Cookie"]["description"]
                            .as_str()
                            .is_some_and(|v| v.contains(name))
                    })
            } else {
                actual["responses"][status]["headers"].get(name).is_some()
            };
            if !preserved {
                return Err(format!("Missing response header {name} for {id}").into());
            }
        }
    }
    Ok(())
}

fn content_types(operation: &Value) -> BTreeSet<&str> {
    operation["requestBody"]["content"]
        .as_object()
        .into_iter()
        .flatten()
        .map(|(name, _)| name.as_str())
        .collect()
}

fn parameters(operation: &Value) -> BTreeSet<(&str, &str, bool)> {
    operation["parameters"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|p| {
            Some((
                p["in"].as_str()?,
                p["name"].as_str()?,
                p["required"].as_bool().unwrap_or(false),
            ))
        })
        .collect()
}

fn error_statuses(operation: &Value) -> BTreeSet<u16> {
    operation["responses"]
        .as_object()
        .into_iter()
        .flatten()
        .filter_map(|(status, _)| status.parse::<u16>().ok())
        .filter(|status| *status >= 400)
        .collect()
}

fn validate_references(document: &Value) -> Result<(), Box<dyn std::error::Error>> {
    fn walk(value: &Value, document: &Value) -> Result<(), Box<dyn std::error::Error>> {
        match value {
            Value::Object(map) => {
                if let Some(reference) = map
                    .get("$ref")
                    .and_then(Value::as_str)
                    .and_then(|r| r.strip_prefix('#'))
                {
                    if document.pointer(reference).is_none() {
                        return Err(format!("Dangling OpenAPI reference: #{reference}").into());
                    }
                }
                for value in map.values() {
                    walk(value, document)?;
                }
            }
            Value::Array(values) => {
                for value in values {
                    walk(value, document)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
    walk(document, document)
}

fn frontend_types(full: &Value) -> Result<String, Box<dyn std::error::Error>> {
    let operations = operation_index(full)?
        .into_iter()
        .map(|(id, op)| {
            format!(
                "  {id}: {{ method: '{}', path: '{}' }},",
                op.method.to_ascii_uppercase(),
                op.path
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    Ok(format!(
        "// @generated by `cargo xtask openapi`; do not edit.\n\
export interface HealthResponse {{ status: 'ok' }}\n\
export interface ReadinessResponse {{\n\
  status: 'ready' | 'not-ready';\n\
  database: boolean;\n\
  docker: boolean;\n\
  setup: boolean;\n\
}}\n\
export const foundationOperations = {{\n\
{operations}\n\
}} as const;\n"
    ))
}

fn write_or_check(
    path: &Path,
    contents: &[u8],
    check: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if check {
        if fs::read(path)? != contents {
            return Err(format!("{} is stale; run `cargo xtask openapi`", path.display()).into());
        }
    } else {
        fs::create_dir_all(path.parent().unwrap())?;
        fs::write(path, contents)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn generated_documents_have_resolvable_references_and_matching_path_parameters() {
        for public in [false, true] {
            let doc = document(public);
            validate_references(&doc).unwrap();
            for (id, operation) in operation_index(&doc).unwrap() {
                let expected: BTreeSet<_> = operation
                    .path
                    .split('/')
                    .filter_map(|p| p.strip_prefix('{').and_then(|p| p.strip_suffix('}')))
                    .collect();
                let actual: BTreeSet<_> = parameters(operation.value)
                    .into_iter()
                    .filter(|(location, _, _)| *location == "path")
                    .map(|(_, name, required)| {
                        assert!(required, "{id}: path parameter is required");
                        name
                    })
                    .collect();
                assert_eq!(actual, expected, "{id}");
            }
        }
    }

    #[test]
    fn frontend_operation_surface_is_preserved() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        verify_frontend_contract(root, &document(false)).unwrap();
    }

    #[test]
    fn generated_artifacts_match_handler_contracts() {
        generate(true).unwrap();
    }

    #[test]
    fn handler_contracts_match_all_dotnet_routes() {
        crate::parity::check().unwrap();
    }

    #[test]
    fn deployment_patch_schema_comes_from_the_request_type() {
        let doc = document(false);
        let schema = &doc["components"]["schemas"]["PatchDeploymentInput"];
        assert_eq!(schema["additionalProperties"], false);
        assert!(
            schema
                .get("required")
                .is_none_or(|v| v.as_array().is_some_and(Vec::is_empty))
        );
        for field in ["id", "name", "description", "platformId", "spec"] {
            assert!(schema["properties"].get(field).is_some(), "missing {field}");
        }
        assert_eq!(
            doc["paths"]["/api/v1/deployments/{id}"]["patch"]["requestBody"]["content"]["application/json"]
                ["schema"],
            json!({"$ref":"#/components/schemas/PatchDeploymentInput"})
        );
    }

    #[test]
    fn automation_patch_and_logs_use_native_dtos() {
        let doc = document(false);
        let schemas = &doc["components"]["schemas"];
        let patch = &schemas["UpdateAutomationActionInput"];
        assert_eq!(patch["additionalProperties"], false);
        assert_eq!(patch["properties"]["code"]["type"], "string");
        assert!(
            patch
                .get("required")
                .is_none_or(|value| value.as_array().is_some_and(Vec::is_empty))
        );
        let metadata = &schemas["UpdateAutomationActionMetadata"];
        assert_eq!(metadata["properties"].as_object().unwrap().len(), 1);
        assert!(metadata["properties"].get("description").is_some());
        assert_eq!(metadata["additionalProperties"], false);
        assert_eq!(
            schemas["AutomationActionRunLogsView"]["properties"]["logs"]["type"],
            "string"
        );
    }

    #[test]
    fn metadata_gate_detects_missing_cookies_examples_and_response_headers() {
        let reference = json!({
            "parameters": [{"name":"refresh_token","in":"cookie","required":true}],
            "requestBody":{"content":{"application/json":{"examples":{"Azure":{"value":{}}}}}},
            "responses":{"200":{"headers":{"refresh_token":{"schema":{"type":"string"}}}}}
        });
        let mut actual = reference.clone();
        actual["responses"] =
            json!({"302":{"headers":{"Set-Cookie":{"description":"Sets refresh_token."}}}});
        verify_operation_metadata("example", &actual, &reference).unwrap();
        let mut missing = actual.clone();
        missing["parameters"] = json!([]);
        assert_ne!(parameters(&missing), parameters(&reference));
        missing = actual.clone();
        missing["requestBody"]["content"]["application/json"]["examples"] = json!({});
        assert!(verify_operation_metadata("example", &missing, &reference).is_err());
        actual["responses"] = json!({});
        assert!(verify_operation_metadata("example", &actual, &reference).is_err());
    }

    #[test]
    fn reference_validation_rejects_missing_nested_components() {
        assert!(validate_references(&json!({"components":{"schemas":{"Broken":{"properties":{"nested":{"$ref":"#/components/schemas/Missing"}}}}}})).is_err());
    }
}
