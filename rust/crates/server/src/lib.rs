#![forbid(unsafe_code)]

pub mod activities_http;
pub mod alerts_http;
pub mod application_info_http;
pub mod automation_http;
pub mod backups_http;
pub mod builds_http;
mod capabilities;
pub mod config;
pub mod contract_router;
pub mod deployments_http;
pub mod diagnostics;
pub mod git_accounts_http;
pub mod git_repositories_http;
pub mod identity_http;
pub mod license_http;
pub mod license_realtime;
pub mod lookup_http;
pub mod metrics;
pub mod mfa_http;
pub mod oidc_http;
pub mod platforms_http;
pub mod profile_http;
pub mod realtime;
pub mod realtime_groups;
pub mod resources_http;
pub mod roles_http;
pub mod service_accounts_http;
pub mod stacks_http;
pub mod swarm_services_http;
pub mod teams_http;
pub mod transport;
pub mod users_http;
pub mod webhooks_http;
pub mod workers;

pub fn automation_endpoint_catalog_json() -> String {
    let endpoints = citadel_contracts::http::ROUTES
        .iter()
        .filter(|route| route.path.starts_with("/api/v1/"))
        .map(|route| {
            let group = route
                .path
                .trim_start_matches("/api/v1/")
                .split('/')
                .next()
                .unwrap_or("api");
            serde_json::json!({
                "key": route.operation_id,
                "group": group,
                "method": match route.method {
                    citadel_contracts::http::HttpMethod::Get => "GET",
                    citadel_contracts::http::HttpMethod::Post => "POST",
                    citadel_contracts::http::HttpMethod::Put => "PUT",
                    citadel_contracts::http::HttpMethod::Patch => "PATCH",
                    citadel_contracts::http::HttpMethod::Delete => "DELETE",
                },
                "path": route.path,
            })
        })
        .collect::<Vec<_>>();
    serde_json::to_string(&endpoints).expect("typed HTTP contracts serialize")
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

#[derive(Serialize)]
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
