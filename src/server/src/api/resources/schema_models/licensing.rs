//! Server-owned OpenAPI descriptions of licensing values.
//! Field types and exhaustive conversions keep these descriptions tied to the domain.

enum_schema!(
    LicenseCapabilitySchema,
    "LicenseCapability",
    citadel_licensing::LicenseCapability,
    [
        CustomAccessControl,
        AutomatedOperations,
        AdvancedAlerting,
        OperationalGuardrails,
        ElasticBuildExecution
    ]
);

enum_schema!(
    LicenseStatusSchema,
    "LicenseStatus",
    citadel_licensing::LicenseStatus,
    [
        Community,
        Valid,
        GracePeriod,
        NotYetValid,
        Expired,
        Invalid,
        InstanceMismatch,
        UnsupportedSchema,
        UnknownSigningKey
    ]
);
