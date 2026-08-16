#![forbid(unsafe_code)]

pub mod application_info_http;
pub mod config;
pub mod diagnostics;
pub mod identity_http;
pub mod metrics;
pub mod profile_http;
pub mod realtime;
pub mod service_accounts_http;
pub mod transport;
pub mod workers;

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
