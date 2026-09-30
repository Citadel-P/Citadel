//! Server-owned OpenAPI descriptions of builds values.
//! Field types and exhaustive conversions keep these descriptions tied to the domain.

enum_schema!(
    BuildRunStatusSchema,
    "BuildRunStatus",
    citadel_builds::BuildRunStatus,
    [
        Queued,
        Preparing,
        Running,
        Succeeded,
        Failed,
        TimedOut,
        Cancelled,
        Rejected,
        Interrupted
    ]
);

enum_schema!(
    BuildAgentPoolValidationStatusSchema,
    "BuildAgentPoolValidationStatus",
    citadel_builds::BuildAgentPoolValidationStatus,
    [NotTested, Ready, Invalid, Degraded]
);
