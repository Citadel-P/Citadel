#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RouteContract {
    pub method: &'static str,
    pub path: &'static str,
    pub operation_id: &'static str,
    pub summary: &'static str,
    pub public: bool,
    pub setup_exempt: bool,
    pub response_schema: Option<&'static str>,
}

pub const ROUTES: &[RouteContract] = &[
    RouteContract {
        method: "get",
        path: "/health",
        operation_id: "getHealth",
        summary: "Process liveness",
        public: true,
        setup_exempt: true,
        response_schema: Some("HealthResponse"),
    },
    RouteContract {
        method: "get",
        path: "/ready",
        operation_id: "getReadiness",
        summary: "Dependency readiness",
        public: false,
        setup_exempt: true,
        response_schema: Some("ReadinessResponse"),
    },
    RouteContract {
        method: "get",
        path: "/metrics",
        operation_id: "getMetrics",
        summary: "OpenMetrics diagnostics",
        public: false,
        setup_exempt: true,
        response_schema: None,
    },
];

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn operation_ids_and_method_paths_are_unique() {
        let mut operations = HashSet::new();
        let mut endpoints = HashSet::new();
        for route in ROUTES {
            assert!(operations.insert(route.operation_id));
            assert!(endpoints.insert((route.method, route.path)));
        }
    }
}
