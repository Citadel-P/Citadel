//! Code-first API documentation, collected from the same factories as the live routers.
use std::sync::LazyLock;
use utoipa::openapi::{
    Components, Info, OpenApi, Paths,
    security::{HttpAuthScheme, HttpBuilder, SecurityScheme},
};

pub(crate) mod compatibility;
pub(crate) mod errors;
pub(crate) mod router;
pub mod serving;

pub fn document(public_only: bool) -> OpenApi {
    let mut api = OpenApi::new(
        Info::new(
            if public_only {
                "Citadel Public API"
            } else {
                "Citadel API"
            },
            "1.0.0",
        ),
        Paths::new(),
    );
    api.components = Some(Components::new());
    api.components.as_mut().unwrap().add_security_scheme(
        "Bearer",
        SecurityScheme::Http(HttpBuilder::new().scheme(HttpAuthScheme::Bearer)
            .bearer_format("Citadel JWT or cit_sa_ Service Account token")
            .description(Some("Use a Citadel User JWT or Service Account bearer token in the Authorization header."))
            .build()),
    );
    api.merge(crate::automation_http::documented_routes().into_openapi());
    api.merge(crate::stacks_http::documented_routes().into_openapi());
    api.merge(crate::license_http::documented_routes().into_openapi());
    api.merge(crate::deployments_http::documented_routes().into_openapi());
    api.merge(crate::activities_http::documented_routes().into_openapi());
    api.merge(crate::service_accounts_http::documented_routes().into_openapi());
    api.merge(crate::teams_http::documented_routes().into_openapi());
    api.merge(crate::profile_http::documented_routes().into_openapi());
    api.merge(crate::users_http::documented_routes().into_openapi());
    api.merge(crate::builds_http::documented_pool_routes().into_openapi());
    api.merge(crate::builds_http::documented_routes().into_openapi());
    api.merge(crate::oidc_http::documented_routes().into_openapi());
    api.merge(crate::search_http::documented_routes().into_openapi());
    api.merge(crate::git_repositories_http::documented_routes().into_openapi());
    api.merge(crate::application_info_http::documented_routes().into_openapi());
    api.merge(crate::actors_http::documented_routes().into_openapi());
    api.merge(crate::backups_http::documented_routes().into_openapi());
    api.merge(crate::mfa_http::documented_routes().into_openapi());
    api.merge(crate::swarm_services_http::documented_routes().into_openapi());
    api.merge(crate::lookup_http::documented_routes().into_openapi());
    api.merge(crate::roles_http::documented_routes().into_openapi());
    api.merge(crate::webhooks_http::documented_routes().into_openapi());
    api.merge(crate::diagnostics_http::documented_routes().into_openapi());
    api.merge(crate::alerts_http::documented_routes().into_openapi());
    api.merge(crate::identity_http::documented_routes().into_openapi());
    api.merge(crate::platforms_http::documented_routes().into_openapi());
    api.merge(crate::git_accounts_http::documented_routes().into_openapi());
    api.merge(crate::resources_http::tags::documented_routes().into_openapi());
    api.merge(crate::resources_http::bindings::documented_routes().into_openapi());
    api.merge(crate::resources_http::catalog::documented_routes().into_openapi());
    for (name, schema) in compatibility::schemas() {
        api.components
            .as_mut()
            .unwrap()
            .schemas
            .entry(name)
            .or_insert(schema);
    }
    if public_only {
        for item in api.paths.paths.values_mut() {
            item.get = item.get.take().filter(is_public);
            item.post = item.post.take().filter(is_public);
            item.put = item.put.take().filter(is_public);
            item.patch = item.patch.take().filter(is_public);
            item.delete = item.delete.take().filter(is_public);
            item.head = item.head.take().filter(is_public);
            item.options = item.options.take().filter(is_public);
            item.trace = item.trace.take().filter(is_public);
        }
        api.paths.paths.retain(|_, item| {
            [
                &item.get,
                &item.post,
                &item.put,
                &item.patch,
                &item.delete,
                &item.head,
                &item.options,
                &item.trace,
            ]
            .iter()
            .any(|op| op.is_some())
        });
    }
    restore_parameter_defaults(&mut api);
    retain_referenced_schemas(&mut api);
    api
}

fn is_public(operation: &utoipa::openapi::path::Operation) -> bool {
    operation
        .extensions
        .as_ref()
        .and_then(|e| e.get("x-citadel-public"))
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
}

pub fn json_document(public_only: bool) -> &'static str {
    static FULL: LazyLock<String> = LazyLock::new(|| {
        document(false)
            .to_pretty_json()
            .expect("OpenAPI serializes")
    });
    static PUBLIC: LazyLock<String> =
        LazyLock::new(|| document(true).to_pretty_json().expect("OpenAPI serializes"));
    if public_only { &PUBLIC } else { &FULL }
}

pub fn setup_exempt(path: &str) -> bool {
    static EXEMPT: LazyLock<Vec<String>> = LazyLock::new(|| {
        document(false)
            .paths
            .paths
            .into_iter()
            .filter_map(|(path, item)| {
                [&item.get, &item.post, &item.put, &item.patch, &item.delete]
                    .iter()
                    .any(|op| {
                        op.as_ref()
                            .and_then(|op| op.extensions.as_ref())
                            .and_then(|e| e.get("x-citadel-setup-exempt"))
                            .and_then(serde_json::Value::as_bool)
                            .unwrap_or(false)
                    })
                    .then_some(path)
            })
            .collect()
    });
    EXEMPT.iter().any(|exempt| exempt == path)
}

// Utoipa's inline parameter attributes support extensions but not defaults.
// Keep defaults beside their handler and emit them as standard schema metadata.
fn restore_parameter_defaults(api: &mut OpenApi) {
    use utoipa::openapi::{RefOr, schema::Schema};
    for item in api.paths.paths.values_mut() {
        for op in [
            &mut item.get,
            &mut item.post,
            &mut item.put,
            &mut item.patch,
            &mut item.delete,
            &mut item.head,
            &mut item.options,
            &mut item.trace,
        ]
        .into_iter()
        .flatten()
        {
            for parameter in op.parameters.iter_mut().flatten() {
                let default = parameter
                    .extensions
                    .as_mut()
                    .and_then(|extensions| extensions.remove("x-citadel-default"));
                if let (Some(value), Some(RefOr::T(Schema::Object(schema)))) =
                    (default, parameter.schema.as_mut())
                {
                    schema.default = Some(value);
                }
            }
        }
    }
}

fn retain_referenced_schemas(api: &mut OpenApi) {
    use std::collections::BTreeSet;
    fn references(value: &serde_json::Value, names: &mut BTreeSet<String>) {
        match value {
            serde_json::Value::Object(object) => {
                if let Some(name) = object
                    .get("$ref")
                    .and_then(serde_json::Value::as_str)
                    .and_then(|r| r.strip_prefix("#/components/schemas/"))
                {
                    names.insert(name.to_owned());
                }
                for value in object.values() {
                    references(value, names);
                }
            }
            serde_json::Value::Array(array) => {
                for value in array {
                    references(value, names);
                }
            }
            _ => {}
        }
    }
    let Some(components) = api.components.as_mut() else {
        return;
    };
    let mut pending = BTreeSet::new();
    references(
        &serde_json::to_value(&api.paths).expect("paths serialize"),
        &mut pending,
    );
    let mut reachable = BTreeSet::new();
    while let Some(name) = pending.pop_first() {
        if !reachable.insert(name.clone()) {
            continue;
        }
        if let Some(schema) = components.schemas.get(&name) {
            references(
                &serde_json::to_value(schema).expect("schema serializes"),
                &mut pending,
            );
        }
    }
    components
        .schemas
        .retain(|name, _| reachable.contains(name));
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    #[test]
    fn full_and_public_documents_preserve_exposure_and_security() {
        let full = serde_json::to_value(document(false)).unwrap();
        let public = serde_json::to_value(document(true)).unwrap();
        let operations = |doc: &Value| {
            doc["paths"]
                .as_object()
                .unwrap()
                .values()
                .flat_map(|item| item.as_object().unwrap().values())
                .filter(|op| op["operationId"].is_string())
                .count()
        };
        assert_eq!(operations(&full), 404);
        assert_eq!(operations(&public), 305);
        assert!(
            public["paths"]
                .get("/api/v1/authentication/login")
                .is_none()
        );
        assert_eq!(
            full["paths"]["/api/v1/users"]["get"]["security"],
            json!([{"Bearer": []}])
        );
        assert!(setup_exempt("/health"));
        assert!(setup_exempt("/api/v1/setup/status"));
        assert!(!setup_exempt("/api/v1/users"));
    }

    #[test]
    fn typed_schemas_override_compatibility_and_preserve_patch_semantics() {
        let doc = serde_json::to_value(document(false)).unwrap();
        let schema = &doc["components"]["schemas"]["PatchDeploymentInput"];
        assert!(schema["properties"].get("name").is_some());
        assert!(schema["required"].as_array().is_none_or(Vec::is_empty));
        assert_eq!(schema["additionalProperties"], false);
        let spec = &doc["components"]["schemas"]["DeploymentSpec"];
        assert_eq!(spec["required"], json!(["image"]));
        assert_eq!(spec["properties"]["updateBehavior"]["default"], "Disabled");
        let problems = &doc["components"]["schemas"]["ProblemDetails"];
        assert!(problems["properties"].get("traceId").is_some());
        assert!(problems["properties"].get("errors").is_some());
    }

    #[test]
    fn custom_null_deserializers_are_documented_as_nullable() {
        let doc = serde_json::to_value(document(false)).unwrap();
        for (name, field) in [
            ("CreateDeploymentInput", "tagIds"),
            ("AlertRuleInput", "name"),
        ] {
            let property = &doc["components"]["schemas"][name]["properties"][field];
            assert!(
                property["type"]
                    .as_array()
                    .is_some_and(|types| types.contains(&json!("null"))),
                "{name}.{field} accepts null"
            );
        }
    }

    #[test]
    fn query_defaults_are_standard_openapi_schema_defaults() {
        let doc = serde_json::to_value(document(false)).unwrap();
        let parameters = doc["paths"]["/api/v1/users"]["get"]["parameters"]
            .as_array()
            .unwrap();
        let page = parameters.iter().find(|p| p["name"] == "Page").unwrap();
        assert_eq!(page["schema"]["default"], 1);
        assert_eq!(page["schema"]["minimum"].as_f64(), Some(1.0));
    }
    #[test]
    fn automation_webhook_schema_is_derived_from_the_runtime_dto() {
        use utoipa::PartialSchema;
        // Utoipa may wrap an inline enum in allOf when adding its default.
        fn enum_values(schema: &Value) -> Option<&Value> {
            schema.get("enum").or_else(|| {
                ["allOf", "oneOf", "anyOf"]
                    .iter()
                    .filter_map(|key| schema[*key].as_array())
                    .flatten()
                    .find_map(enum_values)
            })
        }
        let native = serde_json::to_value(citadel_resources::RepoWebhookConfig::schema()).unwrap();
        for public in [false, true] {
            let doc = serde_json::to_value(document(public)).unwrap();
            let schemas = &doc["components"]["schemas"];
            assert_eq!(schemas["RepoWebhookConfig"], native);
            assert_eq!(
                enum_values(&native["properties"]["provider"]),
                Some(&json!(["GitHub", "GitLab", "Generic"]))
            );
            assert_eq!(
                enum_values(&native["properties"]["authScheme"]),
                Some(&json!([
                    "GitHubHmacSha256",
                    "GitLabSignedToken",
                    "GitLabLegacyToken",
                    "BearerToken"
                ]))
            );
            for owner in [
                "AutomationActionInput",
                "UpdateAutomationActionInput",
                "AutomationActionView",
            ] {
                let webhook = &schemas[owner]["properties"]["webhook"];
                let alternatives = webhook["oneOf"].as_array().unwrap();
                assert!(
                    alternatives
                        .contains(&json!({"$ref":"#/components/schemas/RepoWebhookConfig"})),
                    "{owner}: {webhook}"
                );
                assert!(
                    alternatives.contains(&json!({"type":"null"})),
                    "{owner}: {webhook}"
                );
            }
        }
    }
}

#[cfg(test)]
mod metadata_tests {
    use super::*;
    use serde_json::{Value, json};

    #[test]
    fn documents_authentication_cookies_security_and_rate_limits() {
        let doc = serde_json::to_value(document(false)).unwrap();
        let bearer = &doc["components"]["securitySchemes"]["Bearer"];
        assert_eq!(bearer["scheme"], "bearer");
        assert!(bearer["bearerFormat"].as_str().unwrap().contains("cit_sa_"));
        assert!(
            bearer["description"]
                .as_str()
                .unwrap()
                .contains("Service Account")
        );
        for (route, method, cookie) in [
            ("refresh", "get", "refresh_token"),
            ("logout", "post", "refresh_token"),
            ("mfa/verify", "post", "citadel_mfa_challenge"),
            ("mfa/setup", "get", "citadel_mfa_setup"),
            ("mfa/setup/confirm", "post", "citadel_mfa_setup"),
        ] {
            let op = &doc["paths"][format!("/api/v1/authentication/{route}")][method];
            assert!(
                op["parameters"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|p| p["name"] == cookie && p["in"] == "cookie" && p["required"] == true)
            );
        }
        for (route, method, status) in [
            ("login", "post", "200"),
            ("refresh", "get", "200"),
            ("logout", "post", "204"),
            ("mfa/verify", "post", "200"),
            ("mfa/setup/confirm", "post", "200"),
            ("oidc/{id}/callback", "get", "302"),
        ] {
            let header = &doc["paths"][format!("/api/v1/authentication/{route}")][method]["responses"]
                [status]["headers"]["Set-Cookie"];
            assert_eq!(header["schema"]["type"], "string", "{route}");
            assert!(
                header["description"]
                    .as_str()
                    .unwrap()
                    .contains("refresh_token")
            );
        }
        for item in doc["paths"].as_object().unwrap().values() {
            for op in item
                .as_object()
                .unwrap()
                .values()
                .filter(|v| v.get("operationId").is_some())
            {
                assert!(op["responses"]["429"]["content"]["application/problem+json"].is_object());
            }
        }
    }

    #[test]
    fn operations_are_grouped_and_merge_patch_is_explicit() {
        for public in [false, true] {
            let doc = serde_json::to_value(document(public)).unwrap();
            for item in doc["paths"].as_object().unwrap().values() {
                for op in item
                    .as_object()
                    .unwrap()
                    .values()
                    .filter(|op| op.get("operationId").is_some())
                {
                    let tags = op["tags"].as_array().expect("operation has tags");
                    assert!(!tags.is_empty());
                    assert!(!tags.contains(&json!("default")));
                }
            }
            for (path, tag) in [
                ("/api/v1/deployments/{id}", "Deployments"),
                ("/api/v1/stacks/{id}", "Stacks"),
            ] {
                let op = &doc["paths"][path]["patch"];
                assert_eq!(op["tags"], json!([tag]));
                let content = &op["requestBody"]["content"];
                assert!(content["application/merge-patch+json"].is_object());
                assert_eq!(
                    content["application/merge-patch+json"],
                    content["application/json"]
                );
            }
            assert!(
                doc["paths"]["/api/v1/containers/start"]["patch"]["requestBody"]["content"]
                    .get("application/merge-patch+json")
                    .is_none()
            );
        }
    }

    #[test]
    fn registry_examples_are_valid_requests_in_both_documents() {
        for public in [false, true] {
            let doc = serde_json::to_value(document(public)).unwrap();
            for (route, method) in [
                ("/api/v1/registries", "post"),
                ("/api/v1/registries/{id}", "patch"),
            ] {
                let content = doc["paths"][route][method]["requestBody"]["content"]
                    .as_object()
                    .unwrap();
                for media in content.values() {
                    let examples = media["examples"].as_object().unwrap();
                    assert_eq!(examples.len(), 6);
                    for (name, example) in examples {
                        let value: Value = example["value"].clone();
                        let mut create: citadel_resources::NewRegistry =
                            serde_json::from_value(value.clone()).unwrap();
                        create
                            .validate()
                            .unwrap_or_else(|error| panic!("{name}: {error}"));
                        let _: citadel_resources::RegistryPatch =
                            serde_json::from_value(value).unwrap();
                    }
                }
                if method == "patch" {
                    assert!(content.contains_key("application/merge-patch+json"));
                }
            }
            assert_eq!(
                doc["components"]["securitySchemes"]["Bearer"]["type"],
                json!("http")
            );
        }
    }
}
