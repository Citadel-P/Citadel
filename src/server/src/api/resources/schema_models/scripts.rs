//! Server-owned OpenAPI descriptions of scripts values.
//! Field types and exhaustive conversions keep these descriptions tied to the domain.

enum_schema!(
    ScriptRunStatusSchema,
    "ScriptRunStatus",
    citadel_scripts::ScriptRunStatus,
    [
        Queued, Running, Succeeded, Failed, TimedOut, Cancelled, Rejected
    ]
);
