#![forbid(unsafe_code)]
pub mod api;

pub mod application_info_http;

pub mod bootstrap;

pub mod config;

pub mod diagnostics;
pub mod diagnostics_http;

pub mod license_realtime;
pub mod metrics;
pub mod openapi;
pub mod realtime;
pub mod realtime_groups;
mod request_validation;

pub mod transport;
pub mod workers;

pub fn automation_endpoint_catalog_json() -> String {
    api::endpoint_catalog::automation_catalog_json()
}

use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};

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

pub mod tasks;

pub mod updates;
