//! Server-owned OpenAPI descriptions of automation values.
//! Field types and exhaustive conversions keep these descriptions tied to the domain.

enum_schema!(
    AutomationRunStatusSchema,
    "AutomationRunStatus",
    citadel_automation::AutomationRunStatus,
    [
        Queued, Running, Succeeded, Failed, TimedOut, Cancelled, Rejected
    ]
);
