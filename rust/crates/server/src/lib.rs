#![forbid(unsafe_code)]
pub mod api;

pub mod activities_http;
pub mod actors_http;
pub mod alerts_http;
pub mod application_info_http;
pub mod automation_http;

pub mod bootstrap;

mod capabilities;
pub mod config;

pub mod diagnostics;
pub mod diagnostics_http;

pub mod identity_http;
pub mod license_http;
pub mod license_realtime;
pub mod lookup_http;
pub mod metrics;
pub mod mfa_http;
pub mod oidc_http;
pub mod openapi;
pub mod platforms_http;
pub mod profile_http;
pub mod realtime;
pub mod realtime_groups;
mod request_validation;
pub mod resources_http;
pub mod roles_http;
pub mod search_http;
pub mod service_accounts_http;

pub mod teams_http;
pub mod transport;
pub mod users_http;
pub mod webhooks_http;
pub mod workers;

pub fn automation_endpoint_catalog_json() -> String {
    let api: serde_json::Value =
        serde_json::from_str(openapi::json_document(false)).expect("OpenAPI is valid JSON");
    let mut endpoints = Vec::new();
    for (path, item) in api["paths"].as_object().unwrap() {
        if !path.starts_with("/api/v1/") {
            continue;
        }
        for (method, operation) in item.as_object().unwrap() {
            let Some(id) = operation["operationId"].as_str() else {
                continue;
            };
            let group = path
                .trim_start_matches("/api/v1/")
                .split('/')
                .next()
                .unwrap_or("api");
            endpoints.push(serde_json::json!({"key": id, "group": group, "method": method.to_uppercase(), "path": path}));
        }
    }
    serde_json::to_string(&endpoints).expect("HTTP catalog serializes")
}

use std::sync::atomic::{AtomicBool, Ordering};

use serde::Serialize;

#[derive(Default)]
pub struct Readiness {
    database: AtomicBool,
    docker: AtomicBool,
    setup: AtomicBool,
}

impl Readiness {
    pub fn set(&self, database: bool, docker: bool) {
        self.database.store(database, Ordering::Release);
        self.docker.store(docker, Ordering::Release);
    }

    pub fn set_setup(&self, setup: bool) {
        self.setup.store(setup, Ordering::Release);
    }

    #[must_use]
    pub fn is_setup(&self) -> bool {
        self.setup.load(Ordering::Acquire)
    }

    #[must_use]
    pub fn is_database_ready(&self) -> bool {
        self.database.load(Ordering::Acquire)
    }

    #[must_use]
    pub fn snapshot(&self) -> ReadinessResponse {
        let database = self.database.load(Ordering::Acquire);
        let docker = self.docker.load(Ordering::Acquire);
        let setup = self.setup.load(Ordering::Acquire);
        ReadinessResponse {
            status: if database && docker {
                "ready"
            } else {
                "not-ready"
            },
            database,
            docker,
            setup,
        }
    }
}

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReadinessResponse {
    pub status: &'static str,
    pub database: bool,
    pub docker: bool,
    pub setup: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn readiness_requires_database_and_docker() {
        let readiness = Readiness::default();
        readiness.set(true, false);
        assert_eq!(readiness.snapshot().status, "not-ready");
        readiness.set(true, true);
        assert_eq!(readiness.snapshot().status, "ready");
    }
}

pub(crate) mod token_safety;

pub mod runtime_targets;
