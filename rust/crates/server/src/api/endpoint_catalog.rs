//! Shared route metadata for startup policy, Automation and API documentation.
//! Each route factory is visited separately. Only endpoint metadata is retained;
//! full document merging, compatibility schemas and JSON rendering are explicit docs work.
use std::{collections::BTreeMap, sync::LazyLock};
use utoipa::openapi::OpenApi;

pub(crate) fn visit_documents(mut visit: impl FnMut(OpenApi)) {
    visit(crate::api::routes::automation::documented_routes().into_openapi());
    visit(crate::api::routes::stacks::documented_routes().into_openapi());
    visit(crate::api::routes::licensing::documented_routes().into_openapi());
    visit(crate::api::routes::deployments::documented_routes().into_openapi());
    visit(crate::api::routes::activities::documented_routes().into_openapi());
    visit(crate::api::routes::service_accounts::documented_routes().into_openapi());
    visit(crate::api::routes::teams::documented_routes().into_openapi());
    visit(crate::api::routes::profile::documented_routes().into_openapi());
    visit(crate::api::routes::users::documented_routes().into_openapi());
    visit(crate::api::routes::builds::documented_pool_routes().into_openapi());
    visit(crate::api::routes::builds::documented_routes().into_openapi());
    visit(crate::api::routes::oidc::documented_routes().into_openapi());
    visit(crate::api::routes::search::documented_routes().into_openapi());
    visit(crate::api::routes::git_repositories::documented_routes().into_openapi());
    visit(crate::application_info_http::documented_routes().into_openapi());
    visit(crate::api::routes::actors::documented_routes().into_openapi());
    visit(crate::api::routes::backups::documented_routes().into_openapi());
    visit(crate::api::routes::mfa::documented_routes().into_openapi());
    visit(crate::api::routes::swarm_services::documented_routes().into_openapi());
    visit(crate::api::routes::lookup::documented_routes().into_openapi());
    visit(crate::api::routes::roles::documented_routes().into_openapi());
    visit(crate::api::routes::webhooks::documented_routes().into_openapi());
    visit(crate::diagnostics_http::documented_routes().into_openapi());
    visit(crate::api::routes::alerts::documented_routes().into_openapi());
    visit(crate::api::routes::authentication::documented_routes().into_openapi());
    visit(crate::api::routes::platforms::documented_routes().into_openapi());
    visit(crate::api::routes::git_accounts::documented_routes().into_openapi());
    visit(crate::api::routes::tags::documented_routes().into_openapi());
    visit(crate::api::routes::bindings::documented_routes().into_openapi());
    visit(crate::api::routes::registries::documented_routes().into_openapi());
    visit(crate::api::routes::git_repositories::documented_catalog_routes().into_openapi());
}

struct Endpoint {
    path: String,
    method: &'static str,
    operation_id: Option<String>,
    setup_exempt: bool,
}

static ENDPOINTS: LazyLock<Vec<Endpoint>> = LazyLock::new(|| {
    let mut endpoints = BTreeMap::new();
    visit_documents(|document| {
        for (path, item) in document.paths.paths {
            for (method, operation) in [
                ("DELETE", item.delete),
                ("GET", item.get),
                ("HEAD", item.head),
                ("OPTIONS", item.options),
                ("PATCH", item.patch),
                ("POST", item.post),
                ("PUT", item.put),
                ("TRACE", item.trace),
            ] {
                let Some(operation) = operation else {
                    continue;
                };
                let setup_exempt = operation
                    .extensions
                    .as_ref()
                    .and_then(|extensions| extensions.get("x-citadel-setup-exempt"))
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false);
                // Match OpenApi::merge's first-operation-wins rule and path/method order.
                endpoints
                    .entry((path.clone(), method))
                    .or_insert_with(|| Endpoint {
                        path: path.clone(),
                        method,
                        operation_id: operation.operation_id,
                        setup_exempt,
                    });
            }
        }
    });
    endpoints.into_values().collect()
});

pub(crate) fn automation_catalog_json() -> String {
    let endpoints: Vec<_> = ENDPOINTS.iter().filter_map(|endpoint| {
        let path = endpoint.path.strip_prefix("/api/v1/")?;
        let id = endpoint.operation_id.as_ref()?;
        let group = path.split('/').next().unwrap_or("api");
        Some(serde_json::json!({"key": id, "group": group, "method": endpoint.method, "path": endpoint.path}))
    }).collect();
    serde_json::to_string(&endpoints).expect("HTTP catalog serializes")
}

pub(crate) fn setup_exempt(path: &str) -> bool {
    ENDPOINTS.iter().any(|endpoint| {
        endpoint.path == path
            && endpoint.setup_exempt
            && matches!(endpoint.method, "GET" | "POST" | "PUT" | "PATCH" | "DELETE")
    })
}

#[cfg(test)]
mod tests {
    use crate::api::endpoint_catalog::*;

    #[test]
    fn startup_catalog_and_setup_policy_match_the_public_document_contract() {
        let document: serde_json::Value =
            serde_json::from_str(crate::openapi::json_document(false)).unwrap();
        let mut expected = Vec::new();
        for (path, item) in document["paths"].as_object().unwrap() {
            let mut exempt = false;
            for (method, operation) in item.as_object().unwrap() {
                exempt |= matches!(method.as_str(), "get" | "post" | "put" | "patch" | "delete")
                    && operation["x-citadel-setup-exempt"] == true;
                if path.starts_with("/api/v1/")
                    && let Some(id) = operation["operationId"].as_str()
                {
                    let group = path
                        .trim_start_matches("/api/v1/")
                        .split('/')
                        .next()
                        .unwrap_or("api");
                    expected.push(serde_json::json!({"key": id, "group": group, "method": method.to_uppercase(), "path": path}));
                }
            }
            assert_eq!(setup_exempt(path), exempt, "{path}");
        }
        assert_eq!(
            automation_catalog_json(),
            serde_json::to_string(&expected).unwrap()
        );
        assert!(!setup_exempt("/api/v1/unknown"));
    }
}
