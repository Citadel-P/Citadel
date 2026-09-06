use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use citadel_contracts::http::{
    ErrorResponse, ParameterContract, ParameterSchema, ROUTES, RouteAuthentication, RouteContract,
};
use serde_json::{Map, Value, json};

pub fn generate(check: bool) -> Result<(), Box<dyn std::error::Error>> {
    let rust_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask must be inside the Rust workspace");
    let generated = rust_root.join("generated");
    write_or_check(
        &generated.join("openapi/v1.json"),
        format!("{}\n", serde_json::to_string_pretty(&document(false))?).as_bytes(),
        check,
    )?;
    write_or_check(
        &generated.join("openapi/public-v1.json"),
        format!("{}\n", serde_json::to_string_pretty(&document(true))?).as_bytes(),
        check,
    )?;
    write_or_check(
        &generated.join("frontend/foundation-api.ts"),
        frontend_types().as_bytes(),
        check,
    )?;
    if check {
        verify_frontend_contract(rust_root)?;
    }
    println!(
        "{} {} full and {} public HTTP contracts",
        if check { "verified" } else { "generated" },
        ROUTES.len(),
        ROUTES.iter().filter(|route| route.public).count()
    );
    Ok(())
}

fn verify_frontend_contract(rust_root: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let path = rust_root
        .parent()
        .expect("Rust workspace must be inside the repository")
        .join("src/Citadel.FrontEnd/src/api/schema/swagger.json");
    let frontend: Value = serde_json::from_slice(&fs::read(&path).map_err(|error| {
        format!(
            "cannot read frontend API contract {}: {error}",
            path.display()
        )
    })?)?;
    let frontend_operations = operation_index(&frontend)?;
    let rust = document(false);

    for route in ROUTES
        .iter()
        .filter(|route| route.path.starts_with("/api/v1/"))
    {
        let Some(frontend_operation) = frontend_operations.get(route.operation_id) else {
            return Err(format!(
                "Rust operation '{}' is missing from the generated frontend API contract",
                route.operation_id
            )
            .into());
        };
        let expected_method = route.method.as_openapi_str();
        if frontend_operation.method != expected_method || frontend_operation.path != route.path {
            return Err(format!(
                "frontend route mismatch for '{}': Rust uses {} {}, frontend uses {} {}",
                route.operation_id,
                expected_method.to_ascii_uppercase(),
                route.path,
                frontend_operation.method.to_ascii_uppercase(),
                frontend_operation.path
            )
            .into());
        }

        let rust_operation = &rust["paths"][route.path][expected_method];
        compare_operation_schema(
            route.operation_id,
            "request",
            request_schema(rust_operation),
            &rust,
            request_schema(&frontend_operation.operation),
            &frontend,
        )?;
        compare_operation_schema(
            route.operation_id,
            "response",
            success_schema(rust_operation),
            &rust,
            success_schema(&frontend_operation.operation),
            &frontend,
        )?;

        let rust_error_statuses = error_statuses(rust_operation);
        let frontend_error_statuses = error_statuses(&frontend_operation.operation);
        if !frontend_error_statuses.is_subset(&rust_error_statuses) {
            return Err(format!(
                "frontend error response mismatch for '{}': Rust declares {rust_error_statuses:?}, frontend declares {frontend_error_statuses:?}",
                route.operation_id
            )
            .into());
        }

        let rust_parameters = parameters(rust_operation);
        let frontend_parameters = parameters(&frontend_operation.operation);
        if rust_parameters != frontend_parameters {
            return Err(format!(
                "frontend parameter mismatch for '{}': Rust declares {rust_parameters:?}, frontend declares {frontend_parameters:?}",
                route.operation_id
            )
            .into());
        }
    }
    Ok(())
}

fn error_statuses(operation: &Value) -> BTreeSet<u16> {
    operation["responses"]
        .as_object()
        .into_iter()
        .flatten()
        .filter_map(|(status, _)| status.parse::<u16>().ok())
        .filter(|status| *status >= 400)
        .collect()
}

struct IndexedOperation {
    method: String,
    path: String,
    operation: Value,
}

fn operation_index(
    document: &Value,
) -> Result<BTreeMap<String, IndexedOperation>, Box<dyn std::error::Error>> {
    let paths = document["paths"]
        .as_object()
        .ok_or("frontend API contract has no paths object")?;
    let mut operations = BTreeMap::new();
    for (path, path_item) in paths {
        let Some(methods) = path_item.as_object() else {
            continue;
        };
        for (method, operation) in methods {
            let Some(operation_id) = operation["operationId"].as_str() else {
                continue;
            };
            if operations
                .insert(
                    operation_id.to_owned(),
                    IndexedOperation {
                        method: method.to_owned(),
                        path: path.to_owned(),
                        operation: operation.clone(),
                    },
                )
                .is_some()
            {
                return Err(format!(
                    "frontend API contract contains duplicate operation ID '{operation_id}'"
                )
                .into());
            }
        }
    }
    Ok(operations)
}

fn request_schema(operation: &Value) -> Option<&Value> {
    operation.pointer("/requestBody/content/application~1json/schema")
}

fn success_schema(operation: &Value) -> Option<&Value> {
    operation["responses"]
        .as_object()?
        .iter()
        .filter(|(status, _)| status.starts_with('2'))
        .min_by_key(|(status, _)| status.as_str())
        .and_then(|(_, response)| response.pointer("/content/application~1json/schema"))
}

fn compare_operation_schema(
    operation_id: &str,
    kind: &str,
    rust_schema: Option<&Value>,
    rust_document: &Value,
    frontend_schema: Option<&Value>,
    frontend_document: &Value,
) -> Result<(), Box<dyn std::error::Error>> {
    let rust_reference = rust_schema.and_then(|schema| schema["$ref"].as_str());
    let frontend_reference = frontend_schema.and_then(|schema| schema["$ref"].as_str());
    if rust_reference.is_some() && frontend_reference.is_some() {
        if rust_reference == frontend_reference {
            return Ok(());
        }
        return Err(format!(
            "frontend {kind} schema mismatch for '{operation_id}': Rust uses {rust_reference:?}, frontend uses {frontend_reference:?}"
        )
        .into());
    }

    let rust_resolved = rust_schema.and_then(|schema| resolve_root_schema(schema, rust_document));
    let frontend_resolved =
        frontend_schema.and_then(|schema| resolve_root_schema(schema, frontend_document));
    if rust_resolved == frontend_resolved {
        return Ok(());
    }
    Err(format!(
        "frontend {kind} schema mismatch for '{operation_id}': Rust and frontend root shapes differ"
    )
    .into())
}

fn resolve_root_schema<'a>(schema: &'a Value, document: &'a Value) -> Option<&'a Value> {
    let Some(reference) = schema["$ref"].as_str() else {
        return Some(schema);
    };
    let schema_name = reference.strip_prefix("#/components/schemas/")?;
    document.pointer(&format!("/components/schemas/{schema_name}"))
}

fn parameters(operation: &Value) -> BTreeSet<(String, String, bool)> {
    operation["parameters"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|parameter| parameter["in"] != "cookie")
        .filter_map(|parameter| {
            Some((
                parameter["in"].as_str()?.to_owned(),
                parameter["name"].as_str()?.to_owned(),
                parameter["required"].as_bool().unwrap_or(false),
            ))
        })
        .collect()
}

fn document(public_only: bool) -> Value {
    let mut paths = Map::new();
    for route in ROUTES.iter().filter(|route| !public_only || route.public) {
        let path = paths
            .entry(route.path.to_owned())
            .or_insert_with(|| Value::Object(Map::new()));
        path.as_object_mut()
            .expect("generated path is an object")
            .insert(route.method.as_openapi_str().to_owned(), operation(route));
    }
    json!({
        "openapi": "3.1.1",
        "info": {
            "title": if public_only { "Citadel Public API" } else { "Citadel API" },
            "version": "1.0.0"
        },
        "paths": paths,
        "components": {
            "schemas": schemas(),
            "securitySchemes": {
                "Bearer": { "type": "http", "scheme": "bearer" }
            }
        }
    })
}

fn schemas() -> Value {
    let mut schemas = json!({
        "HealthResponse": {
            "type": "object",
            "required": ["status"],
            "properties": { "status": { "type": "string", "enum": ["ok"] } }
        },
        "ReadinessResponse": {
            "type": "object",
            "required": ["status", "database", "docker", "setup"],
            "properties": {
                "status": { "type": "string", "enum": ["ready", "not-ready"] },
                "database": { "type": "boolean" },
                "docker": { "type": "boolean" },
                "setup": { "type": "boolean" }
            }
        },
        "ProblemDetails": {
            "type": "object",
            "required": ["type", "title", "status", "requestId"],
            "properties": {
                "type": { "type": "string", "format": "uri-reference" },
                "title": { "type": "string" },
                "status": { "type": "integer" },
                "detail": { "type": ["string", "null"] },
                "requestId": { "type": "string" }
            }
        },
        "SetupStatusView": {
            "type": "object", "required": ["requiresSetup"], "additionalProperties": false,
            "properties": { "requiresSetup": { "type": "boolean" } }
        },
        "InitializeCitadelInput": {
            "type": "object", "required": ["name", "email", "password"], "additionalProperties": false,
            "properties": {
                "name": string(), "email": { "type": "string", "format": "email" }, "password": string()
            }
        },
        "LoginRequest": {
            "type": "object", "required": ["emailOrName", "password"], "additionalProperties": false,
            "properties": { "emailOrName": string(), "password": string() }
        },
        "LoginResponse": {
            "type": "object", "required": ["accessToken", "nextStep"], "additionalProperties": false,
            "properties": {
                "accessToken": nullable_string(),
                "nextStep": { "type": "string", "enum": ["Completed", "VerifyMfa", "EnrollMfa"] }
            }
        },
        "RefreshTokenResponse": {
            "type": "object", "required": ["accessToken"], "additionalProperties": false,
            "properties": { "accessToken": string() }
        },
        "MfaPolicy": {
            "type": "string",
            "enum": ["Optional", "RequiredForAdministrators", "RequiredForAllUsers"]
        },
        "MfaVerificationInput": {
            "type": "object", "additionalProperties": false,
            "properties": {
                "code": { "type": ["string", "null"], "writeOnly": true, "pattern": "^[0-9]{6}$" },
                "recoveryCode": { "type": ["string", "null"], "writeOnly": true, "maxLength": 64 }
            }
        },
        "ConfirmMandatoryMfaSetupInput": {
            "type": "object", "required": ["code"], "additionalProperties": false,
            "properties": { "code": { "type": "string", "writeOnly": true, "pattern": "^[0-9]{6}$" } }
        },
        "StartProfileMfaSetupInput": {
            "type": "object", "required": ["password"], "additionalProperties": false,
            "properties": { "password": { "type": "string", "writeOnly": true, "minLength": 6, "maxLength": 128 } }
        },
        "ConfirmProfileMfaSetupInput": {
            "type": "object", "required": ["code"], "additionalProperties": false,
            "properties": { "code": { "type": "string", "writeOnly": true, "pattern": "^[0-9]{6}$" } }
        },
        "DisableProfileMfaInput": {
            "type": "object", "required": ["password"], "additionalProperties": false,
            "properties": {
                "password": { "type": "string", "writeOnly": true, "minLength": 6, "maxLength": 128 },
                "code": { "type": ["string", "null"], "writeOnly": true, "pattern": "^[0-9]{6}$" },
                "recoveryCode": { "type": ["string", "null"], "writeOnly": true, "maxLength": 64 }
            }
        },
        "RegenerateProfileMfaRecoveryCodesInput": {
            "type": "object", "required": ["password", "code"], "additionalProperties": false,
            "properties": {
                "password": { "type": "string", "writeOnly": true, "minLength": 6, "maxLength": 128 },
                "code": { "type": "string", "writeOnly": true, "pattern": "^[0-9]{6}$" }
            }
        },
        "MandatoryMfaSetupView": {
            "type": "object", "required": ["secret", "otpAuthUri", "expiresAt"], "additionalProperties": false,
            "properties": {
                "secret": { "type": "string", "writeOnly": true },
                "otpAuthUri": { "type": "string", "writeOnly": true },
                "expiresAt": { "type": "string", "format": "date-time" }
            }
        },
        "MandatoryMfaSetupCompleteView": {
            "type": "object", "required": ["accessToken", "recoveryCodes"], "additionalProperties": false,
            "properties": {
                "accessToken": string(),
                "recoveryCodes": { "type": "array", "items": { "type": "string", "writeOnly": true } }
            }
        },
        "MfaVerificationView": {
            "type": "object", "required": ["accessToken"], "additionalProperties": false,
            "properties": { "accessToken": string() }
        },
        "ProfileMfaSetupView": {
            "type": "object", "required": ["secret", "otpAuthUri", "expiresAt"], "additionalProperties": false,
            "properties": {
                "secret": { "type": "string", "writeOnly": true },
                "otpAuthUri": { "type": "string", "writeOnly": true },
                "expiresAt": { "type": "string", "format": "date-time" }
            }
        },
        "ProfileMfaStatusView": {
            "type": "object", "required": ["enabled", "remainingRecoveryCodes", "policy", "canDisable"], "additionalProperties": false,
            "properties": {
                "enabled": { "type": "boolean" },
                "remainingRecoveryCodes": { "type": "integer", "format": "int32", "minimum": 0 },
                "policy": { "$ref": "#/components/schemas/MfaPolicy" },
                "canDisable": { "type": "boolean" }
            }
        },
        "ProfileMfaRecoveryCodesView": {
            "type": "object", "required": ["enabled", "recoveryCodes"], "additionalProperties": false,
            "properties": {
                "enabled": { "type": "boolean" },
                "recoveryCodes": { "type": "array", "items": { "type": "string", "writeOnly": true } }
            }
        },
        "OidcLoginProviderView": {
            "type": "object", "required": ["id", "displayName"], "additionalProperties": false,
            "properties": { "id": uuid(), "displayName": string() }
        },
        "OidcLoginProvidersView": {
            "type": "object", "required": ["providers"], "additionalProperties": false,
            "properties": {
                "providers": { "type": "array", "items": { "$ref": "#/components/schemas/OidcLoginProviderView" } }
            }
        },
        "OidcProviderView": {
            "type": "object",
            "required": ["id", "name", "description", "displayName", "issuer", "clientId", "scopes", "enabled", "autoProvisionUsers", "allowEmailAutoLink", "requireEmailVerified", "allowedEmailDomains", "requiredClaimName", "requiredClaimValues", "defaultRoleId", "hasClientSecret", "createdByActorId", "createdAt", "updatedAt"],
            "additionalProperties": false,
            "properties": {
                "id": uuid(), "name": string(), "description": nullable_string(),
                "displayName": string(), "issuer": string(), "clientId": string(), "scopes": string(),
                "enabled": { "type": "boolean" },
                "autoProvisionUsers": { "type": "boolean" },
                "allowEmailAutoLink": { "type": "boolean" },
                "requireEmailVerified": { "type": "boolean" },
                "allowedEmailDomains": nullable_string(),
                "requiredClaimName": nullable_string(),
                "requiredClaimValues": nullable_string(),
                "defaultRoleId": nullable_uuid(),
                "hasClientSecret": { "type": "boolean" },
                "createdByActorId": uuid(),
                "createdAt": { "type": "string", "format": "date-time" },
                "updatedAt": { "type": "string", "format": "date-time" }
            }
        },
        "OidcProvidersView": {
            "type": "object", "required": ["providers"], "additionalProperties": false,
            "properties": {
                "providers": { "type": "array", "items": { "$ref": "#/components/schemas/OidcProviderView" } }
            }
        },
        "OidcProviderInput": {
            "type": "object",
            "required": ["name", "description", "displayName", "issuer", "clientId", "clientSecret", "scopes", "enabled", "autoProvisionUsers", "allowEmailAutoLink", "requireEmailVerified", "allowedEmailDomains", "requiredClaimName", "requiredClaimValues", "defaultRoleId"],
            "additionalProperties": false,
            "properties": {
                "name": string(), "description": nullable_string(), "displayName": string(),
                "issuer": string(), "clientId": string(),
                "clientSecret": { "type": ["null", "string"], "writeOnly": true },
                "scopes": nullable_string(), "enabled": { "type": "boolean" },
                "autoProvisionUsers": { "type": "boolean" },
                "allowEmailAutoLink": { "type": "boolean" },
                "requireEmailVerified": { "type": "boolean" },
                "allowedEmailDomains": nullable_string(), "requiredClaimName": nullable_string(),
                "requiredClaimValues": nullable_string(), "defaultRoleId": nullable_uuid()
            }
        },
        "UpdateOidcProviderInput": {
            "type": "object", "additionalProperties": false,
            "properties": {
                "name": nullable_string(), "description": nullable_string(),
                "displayName": nullable_string(), "issuer": nullable_string(),
                "clientId": nullable_string(),
                "clientSecret": { "type": ["null", "string"], "writeOnly": true },
                "scopes": nullable_string(), "enabled": { "type": ["null", "boolean"] },
                "autoProvisionUsers": { "type": ["null", "boolean"] },
                "allowEmailAutoLink": { "type": ["null", "boolean"] },
                "requireEmailVerified": { "type": ["null", "boolean"] },
                "allowedEmailDomains": nullable_string(), "requiredClaimName": nullable_string(),
                "requiredClaimValues": nullable_string(), "defaultRoleId": nullable_uuid()
            }
        },
        "PatchResourceMetadata": {
            "type": "object", "required": ["description", "tags"], "additionalProperties": false,
            "properties": {
                "description": string(), "tags": { "type": "array", "items": string() }
            }
        },
        "TestOidcProviderDiscoveryInput": {
            "type": "object", "required": ["providerId", "issuer"], "additionalProperties": false,
            "properties": { "providerId": nullable_uuid(), "issuer": nullable_string() }
        },
        "OidcDiscoveryResultView": {
            "type": "object", "required": ["issuer", "authorizationEndpoint", "tokenEndpoint", "jwksUri"], "additionalProperties": false,
            "properties": {
                "issuer": string(), "authorizationEndpoint": string(),
                "tokenEndpoint": string(), "jwksUri": string()
            }
        },
        "PermissionMatrixResponse": {
            "type": "object",
            "additionalProperties": { "$ref": "#/components/schemas/PermissionMatrixViewItem" }
        },
        "PermissionMatrixViewItem": {
            "type": "object", "required": ["maximumLevel", "specificPermissions", "label", "specificPermissionLabels"], "additionalProperties": false,
            "properties": {
                "maximumLevel": { "$ref": "#/components/schemas/PermissionLevel" },
                "specificPermissions": { "type": "object", "additionalProperties": { "$ref": "#/components/schemas/PermissionLevel" } },
                "label": string(),
                "specificPermissionLabels": { "type": "object", "additionalProperties": string() }
            }
        },
        "ResourceType": {
            "type": "string",
            "enum": ["Platform", "Deployment", "Stack", "Registry", "GitRepository", "GitAccount", "Alert", "AlertChannel", "User", "Team", "Role", "Binding", "Tag", "AutomationAction", "License", "BackupRepository", "BackupPolicy", "Volume", "Build", "BuildAgentPool", "SwarmService", "ServiceAccount"]
        },
        "PermissionLevel": { "type": "string", "enum": ["None", "Read", "Write", "Execute"] },
        "SpecificPermission": {
            "type": "string",
            "enum": ["Logs", "Inspect", "Apply", "Pull", "Terminal", "ResourceBindings", "Releases", "Restore", "Browse", "Download", "ManageNodeAgents", "Use", "ManageCredentials"]
        },
        "ApplicationInfoView": {
            "type": "object", "required": ["name", "version", "informationalVersion", "realtimeTransport"], "additionalProperties": false,
            "properties": {
                "name": { "type": "string", "const": "Citadel" },
                "version": string(),
                "informationalVersion": string(),
                "realtimeTransport": { "type": "string", "enum": ["SignalR", "WebSocketV1"] }
            }
        },
        "LicenseStatus": {
            "type": "string",
            "enum": ["Community", "Valid", "GracePeriod", "NotYetValid", "Expired", "Invalid", "InstanceMismatch", "UnsupportedSchema", "UnknownSigningKey"]
        },
        "LicenseCapability": {
            "type": "string",
            "enum": ["CustomAccessControl", "AutomatedOperations", "AdvancedAlerting", "OperationalGuardrails", "ElasticBuildExecution"]
        },
        "LicenseCapabilityView": {
            "type": "object", "required": ["capability", "enabled"], "additionalProperties": false,
            "properties": {
                "capability": { "$ref": "#/components/schemas/LicenseCapability" },
                "enabled": { "type": "boolean" }
            }
        },
        "LicenseEntitlementsView": {
            "type": "object", "required": ["status", "effectiveEdition", "capabilities"], "additionalProperties": false,
            "properties": {
                "status": { "$ref": "#/components/schemas/LicenseStatus" },
                "effectiveEdition": string(),
                "capabilities": { "type": "array", "items": { "$ref": "#/components/schemas/LicenseCapabilityView" } }
            }
        },
        "LicenseView": {
            "type": "object",
            "required": ["status", "effectiveEdition", "licensedEdition", "instanceId", "licenseSchema", "licenseId", "replacedLicenseId", "customerId", "customerName", "fingerprint", "issuedAt", "notBefore", "expiresAt", "graceUntil", "capabilities", "warnings"],
            "additionalProperties": false,
            "properties": {
                "status": { "$ref": "#/components/schemas/LicenseStatus" },
                "effectiveEdition": string(),
                "licensedEdition": nullable_string(),
                "instanceId": uuid(),
                "licenseSchema": { "type": ["integer", "null"], "format": "int32" },
                "licenseId": nullable_string(),
                "replacedLicenseId": nullable_string(),
                "customerId": nullable_string(),
                "customerName": nullable_string(),
                "fingerprint": nullable_string(),
                "issuedAt": nullable_date_time(),
                "notBefore": nullable_date_time(),
                "expiresAt": nullable_date_time(),
                "graceUntil": nullable_date_time(),
                "capabilities": { "type": "array", "items": { "$ref": "#/components/schemas/LicenseCapabilityView" } },
                "warnings": { "type": "array", "items": string() }
            }
        },
        "InstallLicenseInput": {
            "type": "object", "required": ["license"], "additionalProperties": false,
            "properties": { "license": { "type": "string", "writeOnly": true, "maxLength": 65536 } }
        },
        "LicenseRequestView": {
            "type": "object", "required": ["product", "instanceId", "coreVersion", "generatedAt"], "additionalProperties": false,
            "properties": {
                "product": { "type": "string", "const": "citadel" },
                "instanceId": uuid(),
                "coreVersion": string(),
                "generatedAt": { "type": "string", "format": "date-time" }
            }
        },
        "UpdateCurrentProfileInput": {
            "type": "object", "required": ["displayName"], "additionalProperties": false,
            "properties": { "displayName": string() }
        },
        "UserDateTimeFormat": {
            "type": "string", "enum": ["System", "TwentyFourHour", "TwelveHour"]
        },
        "UserTheme": {
            "type": "string", "enum": ["System", "Light", "Dark"]
        },
        "PatchUserPreferencesInput": {
            "type": "object", "minProperties": 1, "additionalProperties": false,
            "properties": {
                "timeZone": { "type": "string" },
                "dateTimeFormat": { "$ref": "#/components/schemas/UserDateTimeFormat" },
                "theme": { "$ref": "#/components/schemas/UserTheme" }
            }
        },
        "ChangeCurrentPasswordInput": {
            "type": "object", "required": ["currentPassword", "newPassword"], "additionalProperties": false,
            "properties": {
                "currentPassword": { "type": "string", "writeOnly": true, "maxLength": 128 },
                "newPassword": { "type": "string", "writeOnly": true, "minLength": 15, "maxLength": 128 }
            }
        },
        "UserPreferencesView": {
            "type": "object", "required": ["timeZone", "dateTimeFormat", "theme", "isPersisted"],
            "additionalProperties": false,
            "properties": {
                "timeZone": nullable_string(),
                "dateTimeFormat": { "$ref": "#/components/schemas/UserDateTimeFormat" },
                "theme": { "$ref": "#/components/schemas/UserTheme" },
                "isPersisted": { "type": "boolean" }
            }
        },
        "UserSessionSummaryView": {
            "type": "object",
            "required": ["id", "displayName", "userAgent", "ipAddress", "createdAt", "lastSeenAt", "expiresAt", "isCurrent"],
            "additionalProperties": false,
            "properties": {
                "id": uuid(),
                "displayName": string(),
                "userAgent": nullable_string(),
                "ipAddress": nullable_string(),
                "createdAt": { "type": "string", "format": "date-time" },
                "lastSeenAt": { "type": "string", "format": "date-time" },
                "expiresAt": { "type": "string", "format": "date-time" },
                "isCurrent": { "type": "boolean" }
            }
        },
        "UserSessionsView": {
            "type": "object", "required": ["sessions", "canRevokeOtherSessions"],
            "additionalProperties": false,
            "properties": {
                "sessions": { "type": "array", "items": { "$ref": "#/components/schemas/UserSessionSummaryView" } },
                "canRevokeOtherSessions": { "type": "boolean" }
            }
        },
        "RevokeOtherProfileSessionsView": {
            "type": "object", "required": ["count"], "additionalProperties": false,
            "properties": { "count": { "type": "integer", "format": "int64", "minimum": 0 } }
        },
        "ActivityStatus": {
            "type": "string", "enum": ["Success", "Failure", "Warning", "Information"]
        },
        "ActivityResourceType": {
            "type": "string",
            "enum": ["Platform", "Registry", "Deployment", "Stack", "AlertRule", "GitRepository", "OidcProvider", "AutomationAction", "User", "Team", "Role", "License", "Build", "BuildAgentPool", "Volume", "BackupPolicy", "SwarmService", "ServiceAccount"]
        },
        "ActivityEventType": {
            "type": "string",
            "enum": [
                "DeploymentCreated", "DeploymentDuplicated", "DeploymentUpdated", "DeploymentRenamed", "DeploymentDeleted", "DeploymentStarted", "DeploymentStopped", "DeploymentPaused", "DeploymentApplied", "DeploymentDegraded", "DeploymentAdopted",
                "PlatformCreated", "PlatformDeleted", "PlatformConnected", "PlatformDisconnected", "PlatformRenamed", "PlatformNodeAgentLifecycle",
                "RegistryCreated", "RegistryRenamed", "RegistryUpdated", "RegistryDeleted",
                "AlertRuleCreated", "AlertRuleUpdated", "AlertRuleDeleted", "AlertRuleRenamed",
                "GitRepoCreated", "GitRepoUpdated", "GitRepoDeleted", "GitRepoRenamed", "GitRepoPulled", "GitRepoCloned", "GitRepoWebhookReceived",
                "OidcProviderCreated", "OidcProviderUpdated", "OidcProviderRenamed", "OidcProviderDeleted",
                "ActionCreated", "ActionUpdated", "ActionRenamed", "ActionDeleted", "ActionRunQueued", "ActionRunStarted", "ActionRunSucceeded", "ActionRunFailed", "ActionRunTimedOut", "ActionRunCancelled", "ActionRunRejected",
                "StackCreated", "StackDuplicated", "StackUpdated", "StackRenamed", "StackDeleted", "StackStarted", "StackStopped", "StackPaused", "StackApplied", "StackRollback", "StackDegraded", "StackDriftDetected", "StackDriftResolved", "StackReconciliationAttempted", "StackGitUpdateAvailable", "StackGitAutoUpdated", "StackGitAutoDeployFailed", "StackImported", "StackWebhookReceived",
                "InitialAdministratorCreated", "UserProfileUpdated", "UserPreferencesUpdated", "UserPasswordChanged", "UserSessionRevoked", "UserOtherSessionsRevoked", "UserMfaEnabled", "UserMfaDisabled", "UserMfaVerificationFailed", "UserMfaRecoveryCodeUsed", "UserMfaRecoveryCodesRegenerated", "UserMfaResetByAdministrator", "UserCreated", "UserUpdated", "UserRenamed", "UserDeleted",
                "TeamCreated", "TeamUpdated", "TeamRenamed", "TeamDeleted", "RoleCreated", "RoleUpdated", "RoleRenamed", "RoleDeleted",
                "LicenseInstalled", "LicenseReplaced", "LicenseRemoved", "LicenseEnteredGracePeriod", "LicenseExpired", "LicenseValidationFailed", "VolumeContentDownloaded",
                "BuildCreated", "BuildUpdated", "BuildRenamed", "BuildDeleted", "BuildRunQueued", "BuildRunStarted", "BuildRunSucceeded", "BuildRunFailed", "BuildRunTimedOut", "BuildRunCancelled", "BuildWebhookReceived", "BuildAgentPoolCreated", "BuildAgentPoolUpdated", "BuildAgentPoolRenamed", "BuildAgentPoolDeleted", "BuildAgentPoolTested",
                "BackupPolicyCreated", "BackupPolicyUpdated", "BackupPolicyRenamed", "BackupPolicyArchived",
                "SwarmServiceCreated", "SwarmServiceAdopted", "SwarmServiceUpdated", "SwarmServiceRenamed", "SwarmServiceDeleted", "SwarmServiceApplied", "SwarmServiceScaled", "SwarmServiceForceUpdated", "SwarmServiceOperationFailed", "SwarmServiceDuplicated", "SwarmServiceWebhookReceived",
                "ServiceAccountCreated", "ServiceAccountUpdated", "ServiceAccountRenamed", "ServiceAccountEnabled", "ServiceAccountDisabled", "ServiceAccountArchived", "ServiceAccountTokenCreated", "ServiceAccountTokenRevoked"
            ]
        },
        "ActivityEventInfo": {
            "type": "object", "required": ["$type"],
            "properties": { "$type": { "$ref": "#/components/schemas/ActivityEventType" } },
            "additionalProperties": true
        },
        "ActivityView": {
            "type": "object",
            "required": ["id", "platformId", "resourceId", "platformName", "resourceName", "platformStatus", "resourceType", "eventType", "status", "createdAt", "info", "actorId", "actorName", "actorType"],
            "additionalProperties": false,
            "properties": {
                "id": uuid(), "platformId": nullable_uuid(), "resourceId": nullable_uuid(),
                "platformName": string(), "resourceName": string(), "platformStatus": string(),
                "resourceType": { "$ref": "#/components/schemas/ActivityResourceType" },
                "eventType": { "$ref": "#/components/schemas/ActivityEventType" },
                "status": { "$ref": "#/components/schemas/ActivityStatus" },
                "createdAt": { "type": "string", "format": "date-time" },
                "info": { "$ref": "#/components/schemas/ActivityEventInfo" },
                "actorId": uuid(), "actorName": string(), "actorType": { "type": "string", "enum": ["User", "System", "Agent", "ServiceAccount", "Team"] }
            }
        },
        "LatestActivityView": {
            "type": "object",
            "required": ["id", "resourceType", "eventType", "status", "info", "createdAt"],
            "additionalProperties": false,
            "properties": {
                "id": uuid(),
                "resourceType": { "$ref": "#/components/schemas/ActivityResourceType" },
                "eventType": { "$ref": "#/components/schemas/ActivityEventType" },
                "status": { "$ref": "#/components/schemas/ActivityStatus" },
                "info": { "$ref": "#/components/schemas/ActivityEventInfo" },
                "createdAt": { "type": "string", "format": "date-time" }
            }
        },
        "PagedActivityView": {
            "type": "object", "required": ["items", "totalCount", "page", "pageSize"], "additionalProperties": false,
            "properties": {
                "items": { "type": "array", "items": { "$ref": "#/components/schemas/ActivityView" } },
                "totalCount": { "type": "integer", "format": "int64", "minimum": 0 },
                "page": { "type": "integer", "format": "int32", "minimum": 1 },
                "pageSize": { "type": "integer", "format": "int32", "minimum": 1, "maximum": 500 }
            }
        },
        "ActivitiesView": {
            "type": "object", "required": ["pagedResult"], "additionalProperties": false,
            "properties": { "pagedResult": { "$ref": "#/components/schemas/PagedActivityView" } }
        },
        "ProfileResourceInfo": {
            "type": "object", "required": ["id", "name"], "additionalProperties": false,
            "properties": { "id": uuid(), "name": string() }
        },
        "CurrentProfileAuthenticationType": {
            "type": "string", "enum": ["Local", "Oidc"]
        },
        "CurrentProfileAuthenticationView": {
            "type": "object",
            "required": ["type", "label", "canChangePassword", "canUseLocalPasswordMfa", "oidcProviderId", "oidcProviderName"],
            "additionalProperties": false,
            "properties": {
                "type": { "$ref": "#/components/schemas/CurrentProfileAuthenticationType" },
                "label": string(),
                "canChangePassword": { "type": "boolean" },
                "canUseLocalPasswordMfa": { "type": "boolean" },
                "oidcProviderId": nullable_uuid(),
                "oidcProviderName": nullable_string()
            }
        },
        "CurrentProfileAuthorizationView": {
            "type": "object", "required": ["isAdministrator", "alertRules", "bindings", "tags"],
            "additionalProperties": false,
            "properties": {
                "isAdministrator": { "type": "boolean" },
                "alertRules": { "$ref": "#/components/schemas/ResourceCapabilities" },
                "bindings": { "$ref": "#/components/schemas/ResourceCapabilities" },
                "tags": { "$ref": "#/components/schemas/ResourceCapabilities" }
            }
        },
        "CurrentProfileView": {
            "type": "object",
            "required": ["id", "displayName", "email", "authentication", "authorization", "createdAt", "directRoles", "teams"],
            "additionalProperties": false,
            "properties": {
                "id": uuid(), "displayName": string(), "email": { "type": "string", "format": "email" },
                "authentication": { "$ref": "#/components/schemas/CurrentProfileAuthenticationView" },
                "authorization": { "$ref": "#/components/schemas/CurrentProfileAuthorizationView" },
                "createdAt": { "type": "string", "format": "date-time" },
                "directRoles": { "type": "array", "items": { "$ref": "#/components/schemas/ProfileResourceInfo" } },
                "teams": { "type": "array", "items": { "$ref": "#/components/schemas/ProfileResourceInfo" } }
            }
        },
        "ResourceInfo": {
            "type": "object", "required": ["id", "name", "group"], "additionalProperties": false,
            "properties": { "id": uuid(), "name": string(), "group": nullable_string() }
        },
        "LookupResourceType": { "type": "string", "enum": citadel_domain::LookupResourceType::ALL },
        "ResourceAccessView": {
            "type": "object",
            "required": ["resourceType", "resourceId", "resourceName", "permissionLevel", "specificPermissions"],
            "additionalProperties": false,
            "properties": {
                "resourceType": { "$ref": "#/components/schemas/ResourceType" },
                "resourceId": uuid(),
                "resourceName": nullable_string(),
                "permissionLevel": { "$ref": "#/components/schemas/PermissionLevel" },
                "specificPermissions": {
                    "type": ["array", "null"],
                    "items": { "$ref": "#/components/schemas/SpecificPermission" }
                },
                "id": nullable_uuid()
            }
        },
        "UserView": {
            "type": "object",
            "required": ["id", "name", "email", "actorId", "isEnabled"],
            "additionalProperties": false,
            "properties": {
                "id": uuid(), "name": string(), "email": string(), "actorId": uuid(),
                "isEnabled": { "type": "boolean" },
                "teams": { "type": ["array", "null"], "items": { "$ref": "#/components/schemas/ResourceInfo" } },
                "roles": { "type": ["array", "null"], "items": { "$ref": "#/components/schemas/ResourceInfo" } },
                "resourceAccesses": { "type": ["array", "null"], "items": { "$ref": "#/components/schemas/ResourceAccessView" } }
            }
        },
        "PagedUserView": {
            "type": "object", "required": ["items", "totalCount", "page", "pageSize"], "additionalProperties": false,
            "properties": {
                "items": { "type": "array", "items": { "$ref": "#/components/schemas/UserView" } },
                "totalCount": { "type": "integer", "format": "int32", "minimum": 0 },
                "page": { "type": "integer", "format": "int32", "minimum": 0 },
                "pageSize": { "type": "integer", "format": "int32", "minimum": 0, "maximum": 500 }
            }
        },
        "UsersView": {
            "type": "object", "required": ["pagedResult", "capabilities"], "additionalProperties": false,
            "properties": {
                "pagedResult": { "$ref": "#/components/schemas/PagedUserView" },
                "capabilities": { "$ref": "#/components/schemas/ResourceCapabilities" }
            }
        },
        "UserSearchItemView": {
            "type": "object", "required": ["id", "name", "email"], "additionalProperties": false,
            "properties": { "id": uuid(), "name": string(), "email": string() }
        },
        "UserSearchItems": {
            "type": "array", "items": { "$ref": "#/components/schemas/UserSearchItemView" }
        },
        "UserResourceAccessInput": {
            "type": "object",
            "required": ["resourceType", "resourceId", "permissionLevel"],
            "additionalProperties": false,
            "properties": {
                "resourceType": { "$ref": "#/components/schemas/ResourceType" },
                "resourceId": uuid(),
                "permissionLevel": { "$ref": "#/components/schemas/PermissionLevel" },
                "specificPermissions": { "type": "array", "items": { "$ref": "#/components/schemas/SpecificPermission" } }
            }
        },
        "CreateUserInput": {
            "type": "object", "required": ["name", "email", "password"], "additionalProperties": false,
            "properties": {
                "name": string(), "email": { "type": "string", "format": "email" },
                "password": { "type": "string", "writeOnly": true, "minLength": 15, "maxLength": 128 },
                "isEnabled": { "type": "boolean", "default": true },
                "teamIds": uuid_array(), "roleIds": uuid_array(),
                "resourceAccesses": { "type": "array", "items": { "$ref": "#/components/schemas/UserResourceAccessInput" } }
            }
        },
        "PatchUserInput": {
            "type": "object", "additionalProperties": false,
            "properties": {
                "email": { "type": ["string", "null"], "format": "email" },
                "password": { "type": ["string", "null"], "writeOnly": true, "minLength": 15, "maxLength": 128 },
                "isEnabled": { "type": ["boolean", "null"] },
                "teamIds": { "type": ["array", "null"], "items": uuid() },
                "roleIds": { "type": ["array", "null"], "items": uuid() },
                "resourceAccesses": { "type": ["array", "null"], "items": { "$ref": "#/components/schemas/UserResourceAccessInput" } }
            }
        },
        "AddUserRoleInput": {
            "type": "object", "required": ["roleId"], "additionalProperties": false,
            "properties": { "roleId": uuid() }
        },
        "AddUserResourceAccessInput": {
            "type": "object", "required": ["resourceType", "resourceId", "permissionLevel"], "additionalProperties": false,
            "properties": {
                "resourceType": { "$ref": "#/components/schemas/ResourceType" }, "resourceId": uuid(),
                "permissionLevel": { "$ref": "#/components/schemas/PermissionLevel" },
                "specificPermissions": { "type": "array", "items": { "$ref": "#/components/schemas/SpecificPermission" } }
            }
        },
        "RemoveUserResourceAccessInput": {
            "type": "object", "required": ["resourceType", "resourceId", "permissionLevel"], "additionalProperties": false,
            "properties": {
                "resourceType": { "$ref": "#/components/schemas/ResourceType" }, "resourceId": uuid(),
                "permissionLevel": { "$ref": "#/components/schemas/PermissionLevel" },
                "specificPermissions": { "type": "array", "items": { "$ref": "#/components/schemas/SpecificPermission" } }
            }
        },
        "DeleteUsersInput": {
            "type": "object", "required": ["ids"], "additionalProperties": false,
            "properties": { "ids": uuid_array() }
        },
        "TeamMemberView": {
            "type": "object", "required": ["actorId", "resourceId", "name", "principalType"], "additionalProperties": false,
            "properties": {
                "actorId": uuid(), "resourceId": uuid(), "name": string(),
                "principalType": { "type": "string", "enum": ["User", "ServiceAccount"] }
            }
        },
        "TeamView": {
            "type": "object",
            "required": ["id", "name", "actorId", "isEnabled", "totalMembers"],
            "additionalProperties": false,
            "properties": {
                "id": uuid(), "name": string(), "actorId": uuid(), "isEnabled": { "type": "boolean" },
                "totalMembers": { "type": "integer", "format": "int32", "minimum": 0 },
                "users": { "type": ["array", "null"], "items": { "$ref": "#/components/schemas/ResourceInfo" } },
                "roles": { "type": ["array", "null"], "items": { "$ref": "#/components/schemas/ResourceInfo" } },
                "resourceAccesses": { "type": ["array", "null"], "items": { "$ref": "#/components/schemas/ResourceAccessView" } },
                "members": { "type": ["array", "null"], "items": { "$ref": "#/components/schemas/TeamMemberView" } }
            }
        },
        "PagedTeamView": {
            "type": "object", "required": ["items", "totalCount", "page", "pageSize"], "additionalProperties": false,
            "properties": {
                "items": { "type": "array", "items": { "$ref": "#/components/schemas/TeamView" } },
                "totalCount": { "type": "integer", "format": "int32", "minimum": 0 },
                "page": { "type": "integer", "format": "int32", "minimum": 0 },
                "pageSize": { "type": "integer", "format": "int32", "minimum": 0, "maximum": 500 }
            }
        },
        "TeamsView": {
            "type": "object", "required": ["pagedResult", "capabilities"], "additionalProperties": false,
            "properties": {
                "pagedResult": { "$ref": "#/components/schemas/PagedTeamView" },
                "capabilities": { "$ref": "#/components/schemas/ResourceCapabilities" }
            }
        },
        "TeamSearchItemView": {
            "type": "object", "required": ["id", "name"], "additionalProperties": false,
            "properties": { "id": uuid(), "name": string() }
        },
        "TeamSearchItems": {
            "type": "array", "items": { "$ref": "#/components/schemas/TeamSearchItemView" }
        },
        "TeamResourceAccessInput": {
            "type": "object", "required": ["resourceType", "resourceId", "permissionLevel"], "additionalProperties": false,
            "properties": {
                "resourceType": { "$ref": "#/components/schemas/ResourceType" }, "resourceId": uuid(),
                "permissionLevel": { "$ref": "#/components/schemas/PermissionLevel" },
                "specificPermissions": { "type": "array", "items": { "$ref": "#/components/schemas/SpecificPermission" } }
            }
        },
        "CreateTeamInput": {
            "type": "object", "required": ["name"], "additionalProperties": false,
            "properties": {
                "name": string(), "userIds": uuid_array(), "roleIds": uuid_array(),
                "resourceAccesses": { "type": "array", "items": { "$ref": "#/components/schemas/TeamResourceAccessInput" } }
            }
        },
        "PatchTeamInput": {
            "type": "object", "additionalProperties": false,
            "properties": {
                "isEnabled": { "type": ["boolean", "null"] },
                "userIds": { "type": ["array", "null"], "items": uuid() },
                "roleIds": { "type": ["array", "null"], "items": uuid() },
                "resourceAccesses": { "type": ["array", "null"], "items": { "$ref": "#/components/schemas/TeamResourceAccessInput" } }
            }
        },
        "AddTeamRoleInput": {
            "type": "object", "required": ["roleId"], "additionalProperties": false,
            "properties": { "roleId": uuid() }
        },
        "AddTeamMemberInput": {
            "type": "object", "required": ["memberActorId"], "additionalProperties": false,
            "properties": { "memberActorId": uuid() }
        },
        "AddTeamResourceAccessInput": {
            "type": "object", "required": ["resourceType", "resourceId", "permissionLevel"], "additionalProperties": false,
            "properties": {
                "resourceType": { "$ref": "#/components/schemas/ResourceType" }, "resourceId": uuid(),
                "permissionLevel": { "$ref": "#/components/schemas/PermissionLevel" },
                "specificPermissions": { "type": "array", "items": { "$ref": "#/components/schemas/SpecificPermission" } }
            }
        },
        "RemoveTeamResourceAccessInput": {
            "type": "object", "required": ["resourceType", "resourceId", "permissionLevel"], "additionalProperties": false,
            "properties": {
                "resourceType": { "$ref": "#/components/schemas/ResourceType" }, "resourceId": uuid(),
                "permissionLevel": { "$ref": "#/components/schemas/PermissionLevel" },
                "specificPermissions": { "type": "array", "items": { "$ref": "#/components/schemas/SpecificPermission" } }
            }
        },
        "DeleteTeamsInput": {
            "type": "object", "required": ["ids"], "additionalProperties": false,
            "properties": { "ids": uuid_array() }
        },
        "RoleType": {
            "type": "string", "enum": ["System", "Custom"]
        },
        "PermissionInput": {
            "type": "object", "required": ["resourceType", "permissionLevel", "specificPermissions"], "additionalProperties": false,
            "properties": {
                "resourceType": { "$ref": "#/components/schemas/ResourceType" },
                "permissionLevel": { "$ref": "#/components/schemas/PermissionLevel" },
                "specificPermissions": { "type": ["array", "null"], "items": { "$ref": "#/components/schemas/SpecificPermission" } }
            }
        },
        "PermissionView": {
            "type": "object", "required": ["resourceType", "permissionLevel", "specificPermissions"], "additionalProperties": false,
            "properties": {
                "resourceType": { "$ref": "#/components/schemas/ResourceType" },
                "permissionLevel": { "$ref": "#/components/schemas/PermissionLevel" },
                "specificPermissions": { "type": "array", "items": { "$ref": "#/components/schemas/SpecificPermission" } }
            }
        },
        "RoleView": {
            "type": "object", "required": ["id", "name", "roleType", "permissions"], "additionalProperties": false,
            "properties": {
                "id": uuid(), "name": string(), "roleType": { "$ref": "#/components/schemas/RoleType" },
                "permissions": { "type": "array", "items": { "$ref": "#/components/schemas/PermissionView" } }
            }
        },
        "RolesView": {
            "type": "object", "required": ["roles", "capabilities"], "additionalProperties": false,
            "properties": {
                "roles": { "type": "array", "items": { "$ref": "#/components/schemas/RoleView" } },
                "capabilities": { "$ref": "#/components/schemas/ResourceCapabilities" }
            }
        },
        "RoleInput": {
            "type": "object", "required": ["name", "permissions"], "additionalProperties": false,
            "properties": {
                "name": string(),
                "permissions": { "type": ["array", "null"], "items": { "$ref": "#/components/schemas/PermissionInput" } }
            }
        },
        "PatchRolePermissionsInput": {
            "type": "object", "required": ["permissions"], "additionalProperties": false,
            "properties": {
                "permissions": { "type": "array", "items": { "$ref": "#/components/schemas/PermissionInput" } }
            }
        },
        "RenameResource": {
            "type": "object", "required": ["id", "name"], "additionalProperties": false,
            "properties": { "id": uuid(), "name": string() }
        },
        "DeleteRolesInput": {
            "type": "object", "required": ["ids"], "additionalProperties": false,
            "properties": { "ids": uuid_array() }
        },
        "ServiceAccountResourceAccess": {
            "type": "object",
            "required": ["id", "resourceType", "resourceId", "resourceName", "permissionLevel", "specificPermissions"],
            "additionalProperties": false,
            "properties": {
                "id": nullable_uuid(),
                "resourceType": { "$ref": "#/components/schemas/ResourceType" },
                "resourceId": uuid(),
                "resourceName": nullable_string(),
                "permissionLevel": { "$ref": "#/components/schemas/PermissionLevel" },
                "specificPermissions": { "type": "array", "items": { "$ref": "#/components/schemas/SpecificPermission" } }
            }
        },
        "CreateServiceAccountInput": {
            "type": "object", "required": ["name"], "additionalProperties": false,
            "properties": {
                "name": string(), "description": nullable_string(), "isEnabled": { "type": "boolean" },
                "teamIds": uuid_array(), "roleIds": uuid_array(),
                "resourceAccesses": { "type": "array", "items": { "$ref": "#/components/schemas/ServiceAccountResourceAccessInput" } }
            }
        },
        "PatchServiceAccountInput": {
            "type": "object", "additionalProperties": false,
            "properties": { "description": nullable_string(), "isEnabled": { "type": ["boolean", "null"] } }
        },
        "AddServiceAccountRoleInput": {
            "type": "object", "required": ["roleId"], "additionalProperties": false,
            "properties": { "roleId": uuid() }
        },
        "ServiceAccountResourceAccessInput": {
            "type": "object", "required": ["resourceType", "resourceId", "permissionLevel"], "additionalProperties": false,
            "properties": {
                "resourceType": { "$ref": "#/components/schemas/ResourceType" }, "resourceId": uuid(),
                "permissionLevel": { "$ref": "#/components/schemas/PermissionLevel" },
                "specificPermissions": { "type": "array", "items": { "$ref": "#/components/schemas/SpecificPermission" } }
            }
        },
        "DeleteServiceAccountsInput": {
            "type": "object", "required": ["ids"], "additionalProperties": false,
            "properties": { "ids": uuid_array() }
        },
        "ServiceAccountView": service_account_view(),
        "ResourceCapabilities": capabilities(false),
        "ServiceAccountCapabilities": capabilities(true),
        "ServiceAccountsView": {
            "type": "object", "required": ["pagedResult", "capabilities"], "additionalProperties": false,
            "properties": {
                "pagedResult": paged("ServiceAccountView"),
                "capabilities": { "$ref": "#/components/schemas/ResourceCapabilities" }
            }
        },
        "CreateServiceAccountTokenInput": {
            "type": "object", "required": ["name"], "additionalProperties": false,
            "properties": {
                "name": string(), "expiresAtUtc": nullable_date_time(), "neverExpires": { "type": "boolean" }
            }
        },
        "ServiceAccountTokenView": service_account_token_view(false),
        "CreatedServiceAccountTokenView": service_account_token_view(true),
        "ServiceAccountTokensView": {
            "type": "object", "required": ["pagedResult"], "additionalProperties": false,
            "properties": { "pagedResult": paged("ServiceAccountTokenView") }
        },
        "ServiceAccountLimitsView": {
            "type": "object",
            "required": ["defaultTokenLifetimeDays", "maximumTokenLifetimeDays", "maximumActiveTokensPerAccount"],
            "additionalProperties": false,
            "properties": {
                "defaultTokenLifetimeDays": integer(), "maximumTokenLifetimeDays": integer(),
                "maximumActiveTokensPerAccount": integer()
            }
        },
        "PlatformCapabilities": platform_capabilities(),
        "ImageCapabilities": image_capabilities(),
        "NetworkCapabilities": network_capabilities(),
        "VolumeCapabilities": volume_capabilities(),
        "PlatformView": platform_view(),
        "EdgeAgentEnrollmentView": {"type":"object","required":["enrollmentId","platformId","token","expiresAtUtc","instructions"],"properties":{
            "enrollmentId":uuid(),"platformId":uuid(),"token":string(),"expiresAtUtc":{"type":"string","format":"date-time"},
            "instructions":{"type":"object","required":["coreUrl","environment","agentImage","dockerRunCommand"],"properties":{"coreUrl":string(),"agentImage":string(),"dockerRunCommand":string(),"environment":{"type":"object","additionalProperties":{"type":"string"}}}}
        }},
        "EdgeAgentStatusView": {"type":"object","required":["connectionStatus"],"properties":{
            "connectionStatus":string(),"lastConnectedAtUtc":nullable_string(),"lastDisconnectedAtUtc":nullable_string(),"lastHeartbeatAtUtc":nullable_string(),"lastSeenVersion":nullable_string(),"lastSeenHostname":nullable_string(),"agentFingerprint":nullable_string(),"protocolVersion":{"type":["integer","null"],"format":"int32"},"revokedAtUtc":nullable_string(),"enrollmentExpiresAtUtc":nullable_string()
        }},
        "PlatformType": { "type": "string", "enum": ["Docker", "DockerSwarm", "Kubernetes"] },
        "PlatformConnectorType": { "type": "string", "enum": ["Unknown", "Local", "Agent", "EdgeAgent"] },
        "CreatePlatformInput": {
            "type": "object",
            "required": ["name", "address"],
            "additionalProperties": false,
            "properties": {
                "name": string(),
                "address": nullable_string(),
                "description": nullable_string(),
                "type": { "$ref": "#/components/schemas/PlatformType" },
                "connectorType": { "$ref": "#/components/schemas/PlatformConnectorType" },
                "pruneHistoricalSwarmTaskContainers": { "type": "boolean", "default": true },
                "tagIds": { "type": "array", "items": uuid() }
            }
        },
        "PlatformsView": collection_view("platforms", "PlatformView", "ResourceCapabilities"),
        "ContainerView": container_view(),
        "StatsHours": {"type":"integer","format":"int32","enum":[24,48,72],"default":24},
        "ContainerStatView": {"type":"object","required":["containerId","created","cpuUsage","memoryActive","memoryCache","memoryLimit","rxBytes","txBytes"],"properties":{
            "containerId":uuid(),"created":{"type":"integer","format":"int64"},"cpuUsage":{"type":"number"},"memoryActive":{"type":"number"},"memoryCache":{"type":"number"},"memoryLimit":{"type":"number"},"rxBytes":{"type":"number"},"txBytes":{"type":"number"}}},
        "PlatformStatView": {"type":"object","required":["created","cpuUsage","memoryUsage","rxBytes","txBytes"],"properties":{
            "created":{"type":"integer","format":"int64"},"cpuUsage":{"type":"number"},"memoryUsage":{"type":"number"},"rxBytes":{"type":"number"},"txBytes":{"type":"number"},"diskUsedBytes":{"type":["integer","null"],"format":"int64"},"diskTotalBytes":{"type":["integer","null"],"format":"int64"},"diskUsage":{"type":["number","null"]}}},
        "ContainerStatsView": {"type":"object","required":["stats"],"properties":{"stats":{"type":"array","items":{"$ref":"#/components/schemas/ContainerStatView"}}}},
        "PlatformStatsView": {"type":"object","required":["stats"],"properties":{"stats":{"type":"array","items":{"$ref":"#/components/schemas/PlatformStatView"}}}},
        "StackStatsView": {"type":"object","required":["containers"],"properties":{"containers":{"type":"array","items":{"type":"object","required":["containerId","containerName","stats"],"properties":{"containerId":string(),"containerName":string(),"stats":{"type":"array","items":{"$ref":"#/components/schemas/ContainerStatView"}}}}}}},
        "ContainersView": collection_view("containers", "ContainerView", "PlatformCapabilities"),
        "ImageView": image_view(),
        "ImagesView": collection_view("images", "ImageView", "ImageCapabilities"),
        "DockerNetworkResultView": network_view(),
        "DockerNetworkDetailsView": network_view(),
        "NetworksView": collection_view("networks", "DockerNetworkResultView", "ResourceCapabilities"),
        "DockerVolumeResultView": volume_view(),
        "VolumesView": collection_view("volumes", "DockerVolumeResultView", "ResourceCapabilities"),
        "SwarmNodeView": swarm_node_view(),
        "SwarmNodesView": collection_view("items", "SwarmNodeView", "PlatformCapabilities"),
        "SwarmServiceView": swarm_service_view(),
        "SwarmServiceStatsView": {"type":"object","required":["dockerServiceId","observedTasks","expectedTasks","complete","observedContainerProjectionIds","missingDockerNodeIds","stats"],"properties":{
            "dockerServiceId":string(),"observedTasks":integer(),"expectedTasks":integer(),"complete":{"type":"boolean"},
            "observedContainerProjectionIds":{"type":"array","items":uuid()},"missingDockerNodeIds":{"type":"array","items":string()},
            "oldestSampleAt":nullable_date_time(),"newestSampleAt":nullable_date_time(),"stats":{"type":"array","items":{"$ref":"#/components/schemas/ContainerStatView"}}}},
        "SwarmServicesView": collection_view("items", "SwarmServiceView", "PlatformCapabilities"),
        "SwarmTaskView": swarm_task_view(),
        "SwarmTaskStatsView": {"type":"object","required":["containerProjectionId","dockerContainerId","stats"],"properties":{"containerProjectionId":uuid(),"dockerContainerId":string(),"stats":{"type":"array","items":{"$ref":"#/components/schemas/ContainerStatView"}}}},
        "SwarmTasksView": collection_view("items", "SwarmTaskView", "PlatformCapabilities"),
        "SwarmNetworkView": swarm_network_view(),
        "SwarmNetworksView": collection_view("items", "SwarmNetworkView", "PlatformCapabilities"),
        "SwarmConfigView": swarm_config_view(),
        "SwarmConfigsView": collection_view("items", "SwarmConfigView", "PlatformCapabilities"),
        "SwarmSecretView": swarm_secret_view(),
        "SwarmSecretsView": collection_view("items", "SwarmSecretView", "PlatformCapabilities")
    });
    schemas
        .as_object_mut()
        .expect("schema catalog is an object")
        .extend(phase5_schemas());
    schemas
        .as_object_mut()
        .expect("schema catalog is an object")
        .extend(phase6_deployment_schemas());
    schemas
}

fn phase6_deployment_schemas() -> Map<String, Value> {
    let mut schemas = Map::new();
    schemas.insert(
        "DeploymentIds".into(),
        json!({"type":"array","items":uuid()}),
    );
    schemas.insert(
        "UpdateBehavior".into(),
        json!({"type":"string","enum":["Disabled","Notify","AutoDeploy"]}),
    );
    schemas.insert("DeploymentStatus".into(), json!({"type":"string","enum":["Unknown","Created","Pending","Applying","Healthy","Degraded","Failed","Stopped"]}));
    schemas.insert(
        "ResourceControlState".into(),
        json!({"type":"string","enum":["Idle","Processing"]}),
    );
    schemas.insert(
        "PlatformStatus".into(),
        json!({"type":"string","enum":["Offline","Online"]}),
    );
    schemas.insert("AutoUpdateStatus".into(), json!({"type":"string","enum":["Unknown","UpToDate","UpdateAvailable","Updating","Failed"]}));
    schemas.insert(
        "StopSignal".into(),
        json!({"type":["string","null"],"enum":["SIGTERM","SIGKILL","SIGINT","SIGQUIT",null]}),
    );
    schemas.insert(
        "ContainerRestartPolicy".into(),
        json!({"type":"string","enum":["No","Always","OnFailure","UnlessStopped"]}),
    );
    schemas.insert(
        "DeploymentImageInfo".into(),
        json!({
            "type":"object","required":["$type"],
            "anyOf":[
                {"$ref":"#/components/schemas/DeploymentImageInfoLocalImage"},
                {"$ref":"#/components/schemas/DeploymentImageInfoExternalImage"},
                {"$ref":"#/components/schemas/DeploymentImageInfoBuildImage"}
            ],
            "discriminator":{"propertyName":"$type","mapping":{
                "Local":"#/components/schemas/DeploymentImageInfoLocalImage",
                "External":"#/components/schemas/DeploymentImageInfoExternalImage",
                "Build":"#/components/schemas/DeploymentImageInfoBuildImage"
            }}
        }),
    );
    schemas.insert(
        "DeploymentImageInfoLocalImage".into(),
        json!({
            "type":"object","required":["$type","imageId"],"additionalProperties":false,
            "properties":{"$type":{"type":"string","const":"Local"},"imageId":string()}
        }),
    );
    schemas.insert("DeploymentImageInfoExternalImage".into(), json!({
        "type":"object","required":["$type","registryId","imageTag"],"additionalProperties":false,
        "properties":{"$type":{"type":"string","const":"External"},"registryId":uuid(),"imageTag":string(),"resolvedDigest":nullable_string()}
    }));
    schemas.insert(
        "DeploymentImageInfoBuildImage".into(),
        json!({
            "type":"object","required":["$type","buildProjectId"],"additionalProperties":false,
            "properties":{
                "$type":{"type":"string","const":"Build"},"buildProjectId":uuid(),
                "redeployOnBuild":{"type":"boolean","default":false},
                "resolvedImageReference":nullable_string(),"resolvedDigest":nullable_string(),
                "resolvedBuildRunId":nullable_uuid(),"appliedImageReference":nullable_string(),
                "appliedDigest":nullable_string(),"appliedBuildRunId":nullable_uuid(),
                "appliedAt":nullable_date_time()
            }
        }),
    );
    schemas.insert("ResourceSpec".into(), json!({
        "type":"object","required":["nanoCpus","memoryLimit"],"additionalProperties":false,
        "properties":{"nanoCpus":{"type":["number","null"],"format":"float"},"memoryLimit":{"type":["number","null"],"format":"float"}}
    }));
    schemas.insert("LifeCycleSpec".into(), json!({
        "type":"object","required":["stopTimeout","stopSignal","restartPolicy"],"additionalProperties":false,
        "properties":{"stopTimeout":{"type":["integer","null"],"format":"int32"},"stopSignal":{"$ref":"#/components/schemas/StopSignal"},"restartPolicy":{"$ref":"#/components/schemas/ContainerRestartPolicy"}}
    }));
    schemas.insert(
        "DeploymentSpec".into(),
        json!({
            "type":"object","required":["image"],"additionalProperties":false,
            "properties":{
                "image":{"$ref":"#/components/schemas/DeploymentImageInfo"},
                "updateBehavior":{"$ref":"#/components/schemas/UpdateBehavior","default":"Disabled"},
                "lifeCycleSpec":nullable(json!({"$ref":"#/components/schemas/LifeCycleSpec"})),
                "resourceSpec":nullable(json!({"$ref":"#/components/schemas/ResourceSpec"})),
                "labels":{"type":["object","null"],"additionalProperties":string()},
                "ports":{"type":["array","null"],"items":string()},
                "volumes":{"type":["array","null"],"items":string()},
                "networks":{"type":["array","null"],"items":string()},
                "command":{"type":["array","null"],"items":string()},
                "environmentVariables":{"type":["array","null"],"items":string()}
            }
        }),
    );
    schemas.insert("AutoUpdateState".into(), json!({
        "type":"object","required":["lastCheckedAt","status"],"additionalProperties":false,
        "properties":{"lastCheckedAt":{"type":"string","format":"date-time"},"status":{"$ref":"#/components/schemas/AutoUpdateStatus"},"currentDigest":nullable_string(),"remoteDigest":nullable_string(),"lastError":nullable_string()}
    }));
    schemas.insert("DeploymentCapabilities".into(), json!({
        "type":"object","required":["canViewLogs","canInspect","canOpenTerminal","canPull","canApply","canViewResourceBindings","canRead","canWrite","canExecute"],"additionalProperties":false,
        "properties":{
            "canViewLogs":{"type":"boolean"},"canInspect":{"type":"boolean"},"canOpenTerminal":{"type":"boolean"},
            "canPull":{"type":"boolean"},"canApply":{"type":"boolean"},"canViewResourceBindings":{"type":"boolean"},
            "canRead":{"type":"boolean"},"canWrite":{"type":"boolean"},"canExecute":{"type":"boolean"}
        }
    }));
    schemas.insert("DeploymentView".into(), json!({
        "type":"object","required":["id","name","description","platformId","createdAt","createdByActorId","status","controlState","autoUpdateState","spec","platformStatus"],
        "properties":{
            "id":uuid(),"name":string(),"description":nullable_string(),"platformId":uuid(),
            "createdAt":{"type":"string","format":"date-time"},"createdByActorId":uuid(),
            "status":{"$ref":"#/components/schemas/DeploymentStatus"},"controlState":{"$ref":"#/components/schemas/ResourceControlState"},
            "autoUpdateState":nullable(json!({"$ref":"#/components/schemas/AutoUpdateState"})),
            "spec":{"$ref":"#/components/schemas/DeploymentSpec"},"platformStatus":{"$ref":"#/components/schemas/PlatformStatus"},
            "platformName":nullable_string(),"imageName":nullable_string(),"imageId":nullable_uuid(),
            "containerId":nullable_uuid(),"dockerContainerId":nullable_string(),"dockerImageId":nullable_string(),
            "tags":{"type":"array","items":{"$ref":"#/components/schemas/TagSummaryView"}},
            "latestActivityView":nullable(json!({"$ref":"#/components/schemas/LatestActivityView"})),
            "capabilities":nullable(json!({"$ref":"#/components/schemas/DeploymentCapabilities"}))
        }
    }));
    schemas.insert(
        "DeploymentsView".into(),
        collection_view("deployments", "DeploymentView", "ResourceCapabilities"),
    );
    schemas.insert("DeploymentConfigView".into(), json!({
        "type":"object","required":["id","name","platformId","description","spec"],"additionalProperties":false,
        "properties":{"id":uuid(),"name":string(),"platformId":uuid(),"description":nullable_string(),"spec":{"$ref":"#/components/schemas/DeploymentSpec"}}
    }));
    schemas.insert("DuplicateSourceInput".into(), json!({
        "type":"object","required":["resourceType","resourceId","resourceName"],"additionalProperties":false,
        "properties":{"resourceType":{"$ref":"#/components/schemas/ActivityResourceType"},"resourceId":uuid(),"resourceName":string()}
    }));
    schemas.insert("CreateDeploymentInput".into(), json!({
        "type":"object","required":["name","platformId","description","spec"],"additionalProperties":false,
        "properties":{"name":string(),"platformId":uuid(),"description":nullable_string(),"spec":{"$ref":"#/components/schemas/DeploymentSpec"},"tagIds":{"type":["array","null"],"items":uuid()},"duplicateSource":nullable(json!({"$ref":"#/components/schemas/DuplicateSourceInput"}))}
    }));
    schemas.insert(
        "ApplyDeploymentInput".into(),
        json!({
            "type":"object","required":["id"],"additionalProperties":false,
            "properties":{"id":uuid(),"recreate":{"type":["boolean","null"],"default":false}}
        }),
    );
    schemas.insert(
        "DeploymentApplyError".into(),
        json!({
            "type":"object","required":["code","message"],"additionalProperties":false,
            "properties":{"code":{"type":"integer","format":"int64"},"message":string()}
        }),
    );
    schemas.insert("ImagePullProgress".into(), json!({
        "type":"object","additionalProperties":false,
        "properties":{"current":{"type":["integer","null"],"format":"int64"},"total":{"type":["integer","null"],"format":"int64"},"start":{"type":["integer","null"],"format":"int64"},"units":nullable_string()}
    }));
    schemas.insert(
        "DeploymentStreamItem".into(),
        json!({
            "type":"object","additionalProperties":false,
            "properties":{
                "id":nullable_string(),"status":nullable_string(),"stream":nullable_string(),
                "progressMessage":nullable_string(),"errorMessage":nullable_string(),
                "progress":nullable(json!({"$ref":"#/components/schemas/ImagePullProgress"})),
                "error":nullable(json!({"$ref":"#/components/schemas/DeploymentApplyError"}))
            }
        }),
    );
    schemas.insert(
        "DeploymentStreamItems".into(),
        json!({
            "type":"array","items":{"$ref":"#/components/schemas/DeploymentStreamItem"}
        }),
    );
    schemas.insert(
        "SwarmServiceImageInfo".into(),
        json!({"type":"object","required":["$type"],"properties":{"$type":string()}}),
    );
    schemas.insert("SwarmServiceSpec".into(), json!({"type":"object","required":["image"],"properties":{"image":{"$ref":"#/components/schemas/SwarmServiceImageInfo"}}}));
    schemas.insert("SwarmServiceCapabilities".into(), json!({"type":"object","required":["canViewLogs","canInspect","canApply","canViewResourceBindings","canRead","canWrite","canExecute"],"properties":{"canViewLogs":{"type":"boolean"},"canInspect":{"type":"boolean"},"canApply":{"type":"boolean"},"canViewResourceBindings":{"type":"boolean"},"canRead":{"type":"boolean"},"canWrite":{"type":"boolean"},"canExecute":{"type":"boolean"}}}));
    schemas.insert("ManagedSwarmServiceView".into(), json!({"type":"object","required":["id","platformId","name","description","dockerName","dockerServiceId","spec","health","synchronizationState","controlState","autoUpdateState","appliedImageDigest","hasPendingDesiredChanges","hasRuntimeDrift","rowVersion","createdAt","updatedAt","platformName","platformStatus","runningTaskCount","desiredTaskCount","updateState","updateMessage","currentOperation","tags"],"properties":{"id":uuid(),"platformId":uuid(),"name":string(),"description":nullable_string(),"dockerName":string(),"dockerServiceId":nullable_string(),"spec":{"$ref":"#/components/schemas/SwarmServiceSpec"},"health":string(),"synchronizationState":string(),"controlState":string(),"autoUpdateState":{"$ref":"#/components/schemas/AutoUpdateState"},"appliedImageDigest":nullable_string(),"hasPendingDesiredChanges":{"type":"boolean"},"hasRuntimeDrift":{"type":"boolean"},"rowVersion":{"type":"integer","format":"int64"},"createdAt":{"type":"string","format":"date-time"},"updatedAt":{"type":"string","format":"date-time"},"platformName":nullable_string(),"platformStatus":{"$ref":"#/components/schemas/PlatformStatus"},"runningTaskCount":{"type":["integer","null"],"format":"int32"},"desiredTaskCount":{"type":["integer","null"],"format":"int32"},"updateState":nullable_string(),"updateMessage":nullable_string(),"currentOperation":{"type":["object","null"]},"tags":{"type":"array","items":{"$ref":"#/components/schemas/TagSummaryView"}},"tasks":{"type":["array","null"],"items":{"type":"object"}},"capabilities":{"$ref":"#/components/schemas/SwarmServiceCapabilities"}}}));
    schemas.insert(
        "ManagedSwarmServicesView".into(),
        collection_view(
            "swarmServices",
            "ManagedSwarmServiceView",
            "ResourceCapabilities",
        ),
    );
    schemas.insert("CreateSwarmServiceInput".into(), json!({"type":"object","required":["name","platformId","description","spec"],"properties":{"name":string(),"platformId":uuid(),"description":nullable_string(),"spec":{"$ref":"#/components/schemas/SwarmServiceSpec"},"tagIds":{"type":["array","null"],"items":uuid()},"duplicateSource":{"type":["object","null"]}}}));
    schemas.insert("UpdateSwarmServiceInput".into(), json!({"type":"object","required":["spec","rowVersion"],"properties":{"spec":{"$ref":"#/components/schemas/SwarmServiceSpec"},"rowVersion":{"type":"integer","format":"int64"}}}));
    schemas.insert("ScaleSwarmServiceInput".into(), json!({"type":"object","required":["replicas"],"properties":{"replicas":{"type":"integer","format":"int32"}}}));
    schemas.insert(
        "SwarmServiceIds".into(),
        json!({"type":"array","items":uuid()}),
    );
    schemas.insert("SwarmServiceProgressItem".into(), json!({"type":"object","required":["serviceId","operationId","stage","message"],"properties":{"serviceId":uuid(),"operationId":nullable_uuid(),"stage":string(),"message":string(),"isCompleted":{"type":"boolean","default":false},"isWarning":{"type":"boolean","default":false},"errorMessage":nullable_string()}}));
    schemas.insert(
        "SwarmServiceProgressItems".into(),
        json!({"type":"array","items":{"$ref":"#/components/schemas/SwarmServiceProgressItem"}}),
    );
    schemas.insert(
        "PatchDeploymentInput".into(),
        json!({
            "type":"object","required":["platformId","spec"],"additionalProperties":false,
            "properties":{"platformId":uuid(),"spec":{"$ref":"#/components/schemas/DeploymentSpec"}}
        }),
    );
    schemas.insert(
        "DuplicateDraftWarningView".into(),
        json!({
            "type":"object","required":["code","message"],"additionalProperties":false,
            "properties":{"code":string(),"message":string(),"fieldPath":nullable_string()}
        }),
    );
    schemas.insert("DeploymentDuplicateDraftView".into(), json!({
        "type":"object","required":["draft","warnings"],"additionalProperties":false,
        "properties":{"draft":{"$ref":"#/components/schemas/CreateDeploymentInput"},"warnings":{"type":"array","items":{"$ref":"#/components/schemas/DuplicateDraftWarningView"}}}
    }));
    schemas.insert(
        "StackSource".into(),
        json!({"type":"string","enum":["WebEditor","Git"]}),
    );
    schemas.insert("StackReleaseStatus".into(),json!({"type":"string","enum":["Unknown","Created","Applying","Healthy","Pending","Paused","Degraded","Failed","Stopped","TimedOut"]}));
    schemas.insert("StackSpec".into(),json!({"type":"object","required":["$type"],"properties":{"$type":{"type":"string","enum":["WebEditor","Git"]},"composeFile":string(),"gitRepoId":nullable_uuid(),"branch":nullable_string(),"commitSha":nullable_string(),"updateBehavior":string(),"projectName":nullable_string(),"destroyBeforeDeploy":{"type":"boolean"},"buildImageBindings":{"type":"array","items":{"type":"object"}}}}));
    schemas.insert("StackDriftPolicy".into(),json!({"type":"object","properties":{"mode":{"type":"string","enum":["Disabled","DetectOnly","AutoFix"]},"alertOnDrift":{"type":"boolean"},"markDegraded":{"type":"boolean"},"autoStartStoppedContainers":{"type":"boolean"},"autoResumePausedContainers":{"type":"boolean"},"removeExtraContainers":{"type":"boolean"}}}));
    schemas.insert("StackView".into(),json!({"type":"object","required":["id","name","stackSource","status","rowVersion"],"properties":{"id":uuid(),"name":string(),"description":nullable_string(),"stackSource":{"$ref":"#/components/schemas/StackSource"},"status":{"$ref":"#/components/schemas/StackReleaseStatus"},"platformId":nullable_uuid(),"platformType":string(),"platformStatus":string(),"platformName":nullable_string(),"spec":{"$ref":"#/components/schemas/StackSpec"},"driftPolicy":{"$ref":"#/components/schemas/StackDriftPolicy"},"tags":{"type":"array","items":{"$ref":"#/components/schemas/TagSummaryView"}},"rowVersion":{"type":"integer","format":"int64"}}}));
    schemas.insert(
        "StacksView".into(),
        collection_view("stacks", "StackView", "ResourceCapabilities"),
    );
    schemas.insert("StackConfigView".into(),json!({"type":"object","properties":{"id":uuid(),"name":string(),"platformId":uuid(),"platformType":string(),"description":nullable_string(),"stackSource":{"$ref":"#/components/schemas/StackSource"},"spec":{"$ref":"#/components/schemas/StackSpec"},"driftPolicy":{"$ref":"#/components/schemas/StackDriftPolicy"},"rowVersion":{"type":"integer","format":"int64"}}}));
    schemas.insert("CreateStackInput".into(),json!({"type":"object","required":["name","platformId","stackSource","spec"],"properties":{"name":string(),"platformId":uuid(),"description":nullable_string(),"stackSource":{"$ref":"#/components/schemas/StackSource"},"spec":{"$ref":"#/components/schemas/StackSpec"},"driftPolicy":{"$ref":"#/components/schemas/StackDriftPolicy"},"tagIds":uuid_array(),"duplicateSource":{"type":["object","null"]}}}));
    schemas.insert("PatchStackInput".into(),json!({"type":"object","properties":{"name":nullable_string(),"platformId":nullable_uuid(),"description":nullable_string(),"stackSource":{"$ref":"#/components/schemas/StackSource"},"spec":{"type":"object"},"driftPolicy":{"$ref":"#/components/schemas/StackDriftPolicy"},"rowVersion":{"type":["integer","null"],"format":"int64"}}}));
    schemas.insert("ApplyStackInput".into(),json!({"type":"object","required":["id"],"properties":{"id":uuid(),"recreate":{"type":["boolean","null"]}}}));
    schemas.insert("RollbackStackInput".into(),json!({"type":"object","required":["stackId","releaseId"],"properties":{"stackId":uuid(),"releaseId":uuid()}}));
    schemas.insert("StackIds".into(), json!({"type":"array","items":uuid()}));
    schemas.insert("StackStreamItem".into(),json!({"type":"object","properties":{"type":string(),"message":nullable_string(),"exitCode":{"type":["integer","null"],"format":"int32"},"stackStatus":{"$ref":"#/components/schemas/StackReleaseStatus"},"severity":nullable_string()}}));
    schemas.insert(
        "StackStreamItems".into(),
        json!({"type":"array","items":{"$ref":"#/components/schemas/StackStreamItem"}}),
    );
    schemas.insert("StackDuplicateDraftView".into(),json!({"type":"object","properties":{"draft":{"$ref":"#/components/schemas/CreateStackInput"},"warnings":{"type":"array","items":{"$ref":"#/components/schemas/DuplicateDraftWarningView"}}}}));
    schemas.insert(
        "StackImportKind".into(),
        json!({"type":"string","enum":["ComposeProject","SwarmStack"]}),
    );
    schemas.insert(
        "StackReleasesView".into(),
        json!({"type":"object","required":["releases"],"properties":{"releases":{"type":"array","items":{"type":"object"}}}}),
    );
    schemas.insert("SwarmStackPreflightInput".into(),json!({"type":"object","required":["composeFiles"],"properties":{"composeFiles":{"type":"array","items":string()},"buildImageBindings":{"type":"array","items":{"type":"object"}}}}));
    schemas.insert("SwarmStackCompatibilityReport".into(),json!({"type":"object","required":["isCompatible","issues"],"properties":{"isCompatible":{"type":"boolean"},"issues":{"type":"array","items":{"$ref":"#/components/schemas/SwarmStackCompatibilityIssue"}}}}));
    schemas.insert("SwarmStackCompatibilityIssue".into(),json!({"type":"object","required":["severity","code","message"],"properties":{"severity":{"type":"string","enum":["Warning","Error"]},"code":{"type":"string"},"message":{"type":"string"},"fieldPath":nullable_string()}}));
    schemas.insert("StackDriftReport".into(),json!({"type":"object","required":["stackId","platformId","hasDrift","hasAutoFixableDrift","hasStructuralDrift","drifts"],"properties":{"stackId":uuid(),"platformId":uuid(),"hasDrift":{"type":"boolean"},"hasAutoFixableDrift":{"type":"boolean"},"hasStructuralDrift":{"type":"boolean"},"drifts":{"type":"array","items":{"type":"object"}}}}));
    schemas.insert("StackDriftPolicyInput".into(),json!({"type":"object","properties":{"mode":{"type":["string","null"],"enum":["Disabled","DetectOnly","AutoFix",null]},"alertOnDrift":{"type":["boolean","null"]},"markDegraded":{"type":["boolean","null"]},"autoStartStoppedContainers":{"type":["boolean","null"]},"autoResumePausedContainers":{"type":["boolean","null"]},"removeExtraContainers":{"type":["boolean","null"]}}}));
    schemas.insert("StackReconciliationResult".into(),json!({"type":"object","required":["stackId","status","beforeReport","actions"],"properties":{"stackId":uuid(),"status":{"type":"string","enum":["NoDrift","Reconciled","Partial","RequiresReapply","Disabled","Failed"]},"beforeReport":{"$ref":"#/components/schemas/StackDriftReport"},"afterReport":{"oneOf":[{"$ref":"#/components/schemas/StackDriftReport"},{"type":"null"}]},"actions":{"type":"array","items":{"type":"object"}}}}));
    schemas.insert("StackAdoptionIssue".into(),json!({"type":"object","required":["code","message","severity"],"properties":{"code":string(),"message":string(),"severity":string(),"fieldPath":nullable_string()}}));
    schemas.insert("ComposeProjectRuntimeService".into(),json!({"type":"object","required":["name","containerCount","states"],"properties":{"name":string(),"image":nullable_string(),"containerCount":{"type":"integer","minimum":0},"states":string_array()}}));
    schemas.insert("ComposeProjectImportSourceView".into(),json!({"type":"object","required":["platformId","platformName","projectName","containerIds","containerNames","services"],"properties":{"platformId":uuid(),"platformName":string(),"projectName":string(),"containerIds":string_array(),"containerNames":string_array(),"services":{"type":"array","items":{"$ref":"#/components/schemas/ComposeProjectRuntimeService"}}}}));
    schemas.insert("ComposeProjectStackDraftView".into(),json!({"type":"object","required":["name","platformId","driftPolicy","tagIds"],"properties":{"name":string(),"platformId":uuid(),"description":nullable_string(),"driftPolicy":{"$ref":"#/components/schemas/StackDriftPolicy"},"tagIds":uuid_array()}}));
    schemas.insert("ComposeProjectImportDraftView".into(),json!({"type":"object","required":["importKind","source","draft","issues","runtimeFingerprint"],"properties":{"importKind":string(),"source":{"$ref":"#/components/schemas/ComposeProjectImportSourceView"},"draft":{"$ref":"#/components/schemas/ComposeProjectStackDraftView"},"issues":{"type":"array","items":{"$ref":"#/components/schemas/StackAdoptionIssue"}},"runtimeFingerprint":string()}}));
    schemas.insert("ValidateComposeProjectImportInput".into(),json!({"type":"object","required":["name","stackSource","spec"],"properties":{"name":string(),"stackSource":{"$ref":"#/components/schemas/StackSource"},"spec":{"$ref":"#/components/schemas/StackSpec"},"importKind":{"type":["string","null"]}}}));
    schemas.insert("ComposeProjectServiceComparison".into(),json!({"type":"object","required":["name","runtimeContainerCount","definedInSource"],"properties":{"name":string(),"runtimeContainerCount":{"type":"integer","minimum":0},"runtimeImage":nullable_string(),"definedInSource":{"type":"boolean"},"sourceImage":nullable_string()}}));
    schemas.insert("ComposeProjectImportValidation".into(),json!({"type":"object","required":["services","issues","previewFingerprint","importableSensitiveEnvironmentNames","canImportSensitiveEnvironmentValues"],"properties":{"services":{"type":"array","items":{"$ref":"#/components/schemas/ComposeProjectServiceComparison"}},"issues":{"type":"array","items":{"$ref":"#/components/schemas/StackAdoptionIssue"}},"previewFingerprint":string(),"importableSensitiveEnvironmentNames":string_array(),"canImportSensitiveEnvironmentValues":{"type":"boolean"}}}));
    schemas.insert("ImportComposeProjectInput".into(),json!({"type":"object","required":["name","description","stackSource","spec","previewFingerprint"],"properties":{"name":string(),"description":nullable_string(),"stackSource":{"$ref":"#/components/schemas/StackSource"},"spec":{"$ref":"#/components/schemas/StackSpec"},"previewFingerprint":string(),"tagIds":uuid_array(),"importKind":string(),"importSensitiveEnvironmentAsSecrets":{"type":"boolean"}}}));
    schemas
}

fn phase5_schemas() -> Map<String, Value> {
    let mut schemas = Map::new();
    schemas.insert(
        "ResourceBindingScope".into(),
        json!({"type":"string","enum":["Global","Stack","Deployment","SwarmService"]}),
    );
    schemas.insert(
        "TagView".into(),
        json!({"type":"object","required":["id","name","normalizedName","color","createdByActorId","createdAt","updatedAt","usageCount"],"properties":{"id":uuid(),"name":string(),"normalizedName":string(),"color":string(),"createdByActorId":uuid(),"createdAt":{"type":"string","format":"date-time"},"updatedAt":{"type":"string","format":"date-time"},"usageCount":{"type":"integer","format":"int32","minimum":0},"capabilities":{"$ref":"#/components/schemas/ResourceCapabilities"}}}),
    );
    schemas.insert(
        "TagsView".into(),
        collection_view("tags", "TagView", "ResourceCapabilities"),
    );
    schemas.insert(
        "TagSummaryView".into(),
        json!({"type":"object","required":["id","name","color"],"properties":{"id":uuid(),"name":string(),"color":string()}}),
    );
    schemas.insert(
        "CreateTagInput".into(),
        json!({"type":"object","required":["name","color"],"properties":{"name":string(),"color":string()}}),
    );
    schemas.insert(
        "PatchTagInput".into(),
        json!({"type":"object","properties":{"name":string(),"color":string()}}),
    );
    schemas.insert(
        "ResourceTagsView".into(),
        json!({"type":"object","required":["tags"],"properties":{"tags":{"type":"array","items":{"$ref":"#/components/schemas/TagSummaryView"}}}}),
    );
    schemas.insert(
        "ReplaceResourceTagsInput".into(),
        json!({"type":"object","required":["tagIds"],"properties":{"tagIds":uuid_array()}}),
    );

    let registry = json!({"type":"object","required":["id","name","registryHost","status","type","createdAt","createdByActorId","tags"],"properties":{"id":uuid(),"name":string(),"description":nullable_string(),"registryHost":string(),"status":{"type":"string","enum":["Active","Disabled","Deprecated"]},"type":string(),"createdAt":{"type":"string","format":"date-time"},"createdByActorId":uuid(),"tags":{"type":"array","items":{"$ref":"#/components/schemas/TagSummaryView"}},"capabilities":{"$ref":"#/components/schemas/ResourceCapabilities"},"isDefault":{"type":"boolean"}}});
    schemas.insert("RegistryView".into(), registry);
    schemas.insert(
        "RegistriesView".into(),
        collection_view("registries", "RegistryView", "ResourceCapabilities"),
    );
    schemas.insert(
        "RegistryConfigView".into(),
        json!({"type":"object","properties":{"id":uuid(),"name":string(),"description":nullable_string(),"registryHost":string(),"status":string(),"configuration":{"type":"object"},"tags":{"type":"array","items":{"$ref":"#/components/schemas/TagSummaryView"}}}}),
    );
    schemas.insert("CreateRegistryInput".into(), catalog_input_schema(true));
    schemas.insert("PatchRegistryInput".into(), catalog_patch_schema(true));
    schemas.insert("DeleteRegistriesInput".into(), ids_input_schema());

    schemas.insert(
        "GitTransport".into(),
        json!({"type":"string","enum":["Http","Https","Ssh"]}),
    );
    schemas.insert(
        "GitAuthType".into(),
        json!({"type":"string","enum":["Basic","Token","SshKey"]}),
    );
    schemas.insert("GitAuthConfigurationBasicAuth".into(), json!({"type":"object","required":["$type","username","password"],"additionalProperties":false,"properties":{"$type":{"type":"string","const":"Basic"},"username":string(),"password":{"type":"string","writeOnly":true}}}));
    schemas.insert("GitAuthConfigurationTokenAuth".into(), json!({"type":"object","required":["$type","token"],"additionalProperties":false,"properties":{"$type":{"type":"string","const":"Token"},"token":{"type":"string","writeOnly":true}}}));
    schemas.insert("GitAuthConfigurationSshKeyAuth".into(), json!({"type":"object","required":["$type","username","privateKey","passphrase"],"additionalProperties":false,"properties":{"$type":{"type":"string","const":"SshKey"},"username":string(),"privateKey":{"type":"string","writeOnly":true},"passphrase":{"type":["string","null"],"writeOnly":true}}}));
    schemas.insert("GitAuthConfiguration".into(), json!({"oneOf":[{"$ref":"#/components/schemas/GitAuthConfigurationBasicAuth"},{"$ref":"#/components/schemas/GitAuthConfigurationTokenAuth"},{"$ref":"#/components/schemas/GitAuthConfigurationSshKeyAuth"}],"discriminator":{"propertyName":"$type","mapping":{"Basic":"#/components/schemas/GitAuthConfigurationBasicAuth","Token":"#/components/schemas/GitAuthConfigurationTokenAuth","SshKey":"#/components/schemas/GitAuthConfigurationSshKeyAuth"}}}));
    schemas.insert("GitAccountInput".into(), json!({"type":"object","required":["name","domain","transport","authType","configuration"],"additionalProperties":false,"properties":{"name":string(),"domain":string(),"transport":{"$ref":"#/components/schemas/GitTransport"},"authType":{"$ref":"#/components/schemas/GitAuthType"},"configuration":{"$ref":"#/components/schemas/GitAuthConfiguration"}}}));
    schemas.insert("GitAccountPatch".into(), json!({"type":"object","additionalProperties":false,"properties":{"name":string(),"domain":string(),"transport":{"$ref":"#/components/schemas/GitTransport"},"authType":{"$ref":"#/components/schemas/GitAuthType"},"configuration":{"$ref":"#/components/schemas/GitAuthConfiguration"}}}));
    schemas.insert("GitAccountView".into(), json!({"type":"object","required":["id","createdByActorId","name","domain","transport","authType","createdAt"],"properties":{"id":uuid(),"createdByActorId":uuid(),"name":string(),"domain":string(),"transport":{"$ref":"#/components/schemas/GitTransport"},"authType":{"$ref":"#/components/schemas/GitAuthType"},"createdAt":{"type":"string","format":"date-time"},"capabilities":{"$ref":"#/components/schemas/ResourceCapabilities"}}}));
    schemas.insert(
        "GitAccountsView".into(),
        collection_view("gitAccounts", "GitAccountView", "ResourceCapabilities"),
    );
    schemas.insert("GitAccountConfigView".into(), json!({"type":"object","required":["id","name","domain","transport","authType","configuration"],"additionalProperties":false,"properties":{"id":uuid(),"name":string(),"domain":string(),"transport":{"$ref":"#/components/schemas/GitTransport"},"authType":{"$ref":"#/components/schemas/GitAuthType"},"configuration":{"$ref":"#/components/schemas/GitAuthConfiguration"}}}));
    schemas.insert("DeleteGitAccountsInput".into(), ids_input_schema());

    schemas.insert(
        "GitRepositorySyncMode".into(),
        json!({"type":"string","enum":["Manual","PullInterval"]}),
    );
    schemas.insert(
        "RepoCommand".into(),
        json!({"type":"object","required":["commands","path"],"properties":{"commands":string_array(),"path":string()}}),
    );
    schemas.insert(
        "RepoWebhookConfig".into(),
        json!({"type":"object","properties":{"enabled":{"type":"boolean"},"provider":{"type":"string","enum":["GitHub","GitLab","Generic"]},"authScheme":{"type":"string","enum":["GitHubHmacSha256","GitLabSignedToken","GitLabLegacyToken","BearerToken"]},"secret":nullable_string(),"branchFilter":nullable_string()}}),
    );
    let git = json!({"type":"object","required":["id","name","url","defaultBranch","syncMode","status","createdAt","createdByActorId","controlState","tags"],"properties":{"id":uuid(),"name":string(),"description":nullable_string(),"url":string(),"defaultBranch":string(),"gitAccountId":nullable_uuid(),"syncMode":{"$ref":"#/components/schemas/GitRepositorySyncMode"},"syncIntervalMinutes":{"type":["integer","null"],"format":"int32"},"webhook":{"oneOf":[{"type":"null"},{"$ref":"#/components/schemas/RepoWebhookConfig"}]},"onClone":{"oneOf":[{"type":"null"},{"$ref":"#/components/schemas/RepoCommand"}]},"onPull":{"oneOf":[{"type":"null"},{"$ref":"#/components/schemas/RepoCommand"}]},"status":string(),"createdAt":{"type":"string","format":"date-time"},"createdByActorId":uuid(),"controlState":string(),"latestActivityView":{"oneOf":[{"type":"null"},{"$ref":"#/components/schemas/LatestActivityView"}]},"tags":{"type":"array","items":{"$ref":"#/components/schemas/TagSummaryView"}},"capabilities":{"$ref":"#/components/schemas/ResourceCapabilities"}}});
    schemas.insert("GitRepositoryView".into(), git);
    schemas.insert(
        "GitRepositoriesView".into(),
        collection_view(
            "gitRepositories",
            "GitRepositoryView",
            "ResourceCapabilities",
        ),
    );
    schemas.insert(
        "GitRepositoryConfigView".into(),
        json!({"type":"object","properties":{"id":uuid(),"name":string(),"description":nullable_string(),"url":string(),"defaultBranch":string(),"gitAccountId":nullable_uuid(),"syncMode":{"$ref":"#/components/schemas/GitRepositorySyncMode"},"syncIntervalMinutes":{"type":["integer","null"],"format":"int32"},"webhook":{"oneOf":[{"type":"null"},{"$ref":"#/components/schemas/RepoWebhookConfig"}]},"onClone":{"oneOf":[{"type":"null"},{"$ref":"#/components/schemas/RepoCommand"}]},"onPull":{"oneOf":[{"type":"null"},{"$ref":"#/components/schemas/RepoCommand"}]},"tags":{"type":"array","items":{"$ref":"#/components/schemas/TagSummaryView"}}}}),
    );
    schemas.insert(
        "CreateGitRepositoryInput".into(),
        catalog_input_schema(false),
    );
    schemas.insert(
        "PatchGitRepositoryInput".into(),
        catalog_patch_schema(false),
    );
    schemas.insert("DeleteGitRepositoriesInput".into(), ids_input_schema());
    schemas.insert(
        "GitRepositoryRefView".into(),
        json!({"type":"object","required":["id","gitRepositoryId","branch","status","lastSyncedAt"],"properties":{"id":uuid(),"gitRepositoryId":uuid(),"branch":string(),"resolvedCommitSha":nullable_string(),"status":string(),"lastError":nullable_string(),"lastSyncedAt":{"type":"string","format":"date-time"}}}),
    );
    schemas.insert(
        "GitRepositoryRefsView".into(),
        json!({"type":"object","required":["refs"],"properties":{"refs":{"type":"array","items":{"$ref":"#/components/schemas/GitRepositoryRefView"}}}}),
    );
    schemas.insert(
        "GitRepositoryEntryType".into(),
        json!({"type":"string","enum":["Directory","File","Symlink","Submodule"]}),
    );
    schemas.insert(
        "GitRepositoryEntryView".into(),
        json!({"type":"object","required":["name","path","type","mode"],"properties":{"name":string(),"path":string(),"type":{"$ref":"#/components/schemas/GitRepositoryEntryType"},"size":{"type":["integer","null"],"format":"int64"},"mode":string(),"targetCommitSha":nullable_string()}}),
    );
    schemas.insert(
        "GitRepositoryDirectoryListingView".into(),
        json!({"type":"object","required":["repositoryId","commitSha","path","entries","isTruncated"],"properties":{"repositoryId":uuid(),"commitSha":string(),"path":string(),"entries":{"type":"array","items":{"$ref":"#/components/schemas/GitRepositoryEntryView"}},"isTruncated":{"type":"boolean"},"providerRepositoryUrl":nullable_string()}}),
    );
    schemas.insert(
        "GitRepositoryFileContentView".into(),
        json!({"type":"object","required":["repositoryId","commitSha","path","type","size","isBinary","isTruncated"],"properties":{"repositoryId":uuid(),"commitSha":string(),"path":string(),"type":{"$ref":"#/components/schemas/GitRepositoryEntryType"},"size":{"type":"integer","format":"int64"},"isBinary":{"type":"boolean"},"isTruncated":{"type":"boolean"},"content":nullable_string(),"previewUnavailableReason":nullable_string(),"providerUrl":nullable_string()}}),
    );
    schemas.insert(
        "GitChangedPathStatus".into(),
        json!({"type":"string","enum":["Added","Modified","Deleted","Renamed","Copied","TypeChanged"]}),
    );
    schemas.insert(
        "GitChangedPathView".into(),
        json!({"type":"object","required":["status","path"],"properties":{"status":{"$ref":"#/components/schemas/GitChangedPathStatus"},"path":string(),"previousPath":nullable_string()}}),
    );
    schemas.insert(
        "GitCommitComparisonView".into(),
        json!({"type":"object","required":["repositoryId","baseCommitSha","headCommitSha","files","isTruncated"],"properties":{"repositoryId":uuid(),"baseCommitSha":string(),"headCommitSha":string(),"files":{"type":"array","items":{"$ref":"#/components/schemas/GitChangedPathView"}},"isTruncated":{"type":"boolean"}}}),
    );
    schemas.insert(
        "GitRepositoryBranchView".into(),
        json!({"type":"object","required":["branch","commitSha"],"properties":{"branch":string(),"commitSha":string()}}),
    );
    schemas.insert(
        "GitRepositoryBranchesView".into(),
        json!({"type":"object","required":["branches"],"properties":{"branches":{"type":"array","items":{"$ref":"#/components/schemas/GitRepositoryBranchView"}}}}),
    );
    schemas.insert(
        "GitComposeProjectCandidate".into(),
        json!({"type":"object","required":["workingDirectory","composePaths","envFilePaths","suggestedWatchPaths"],"properties":{"workingDirectory":string(),"composePaths":string_array(),"envFilePaths":string_array(),"suggestedWatchPaths":string_array()}}),
    );
    schemas.insert(
        "GitRepositoryComposeDiscovery".into(),
        json!({"type":"object","required":["repositoryId","branch","resolvedCommitSha","projects"],"properties":{"repositoryId":uuid(),"branch":string(),"resolvedCommitSha":string(),"projects":{"type":"array","items":{"$ref":"#/components/schemas/GitComposeProjectCandidate"}}}}),
    );
    schemas.insert(
        "PatchResourceMetadata".into(),
        json!({"type":"object","properties":{"description":nullable_string(),"tags":string_array()}}),
    );

    let automation_action = json!({"type":"object","required":["id","name","code","defaultArgsJson","enabled","scheduleEnabled","scheduleTimeZone","timeoutSeconds","alertOnFailure","runAsActorId","controlState","rowVersion","createdByActorId","createdAt","updatedAt"],"properties":{"id":uuid(),"name":string(),"description":nullable_string(),"code":string(),"defaultArgsJson":string(),"enabled":{"type":"boolean"},"scheduleEnabled":{"type":"boolean"},"scheduleCron":nullable_string(),"scheduleTimeZone":string(),"webhook":{"oneOf":[{"type":"null"},{"$ref":"#/components/schemas/RepoWebhookConfig"}]},"timeoutSeconds":{"type":"integer","format":"int32"},"alertOnFailure":{"type":"boolean"},"runAsActorId":uuid(),"controlState":string(),"currentRunId":nullable_uuid(),"rowVersion":{"type":"integer","format":"int64"},"createdByActorId":uuid(),"createdAt":{"type":"string","format":"date-time"},"updatedAt":{"type":"string","format":"date-time"}}});
    schemas.insert("AutomationActionView".into(), automation_action);
    schemas.insert("AutomationActionsView".into(), json!({"type":"object","required":["actions"],"properties":{"actions":{"type":"array","items":{"$ref":"#/components/schemas/AutomationActionView"}}}}));
    schemas.insert("AutomationActionInput".into(), json!({"type":"object","required":["name","code","enabled","scheduleEnabled","alertOnFailure"],"properties":{"name":string(),"description":nullable_string(),"code":string(),"defaultArgsJson":nullable_string(),"enabled":{"type":"boolean"},"scheduleEnabled":{"type":"boolean"},"scheduleCron":nullable_string(),"scheduleTimeZone":nullable_string(),"webhook":{"oneOf":[{"type":"null"},{"$ref":"#/components/schemas/RepoWebhookConfig"}]},"timeoutSeconds":{"type":["integer","null"],"format":"int32"},"alertOnFailure":{"type":"boolean"},"runAsActorId":nullable_uuid(),"tagIds":uuid_array()}}));
    schemas.insert("UpdateAutomationActionInput".into(), json!({"type":"object","additionalProperties":false,"properties":{"description":nullable_string(),"code":nullable_string(),"defaultArgsJson":nullable_string(),"enabled":{"type":["boolean","null"]},"scheduleEnabled":{"type":["boolean","null"]},"scheduleCron":nullable_string(),"scheduleTimeZone":nullable_string(),"webhook":{"oneOf":[{"type":"null"},{"$ref":"#/components/schemas/RepoWebhookConfig"}]},"timeoutSeconds":{"type":["integer","null"],"format":"int32"},"alertOnFailure":{"type":["boolean","null"]},"runAsActorId":nullable_uuid()}}));
    schemas.insert(
        "RunAutomationActionInput".into(),
        json!({"type":"object","properties":{"argsJson":{"type":"object"},"timeoutSeconds":{"type":["integer","null"],"format":"int32"}}}),
    );
    schemas.insert(
        "TestAutomationActionInput".into(),
        json!({"type":"object","properties":{"argsJson":{"type":"object"}}}),
    );
    schemas.insert("AutomationActionRunView".into(), json!({"type":"object","required":["id","actionId","actionName","trigger","status","runAsActorId","argsJson","codeSnapshot","codeHash","timeoutSeconds","queuedAt"],"properties":{"id":uuid(),"actionId":uuid(),"actionName":string(),"trigger":string(),"status":string(),"runAsActorId":uuid(),"triggeredByActorId":nullable_uuid(),"argsJson":{"type":"object"},"codeSnapshot":string(),"codeHash":string(),"timeoutSeconds":{"type":"integer","format":"int32"},"queuedAt":{"type":"string","format":"date-time"},"startedAt":{"type":["string","null"],"format":"date-time"},"finishedAt":{"type":["string","null"],"format":"date-time"},"durationMs":{"type":["integer","null"],"format":"int64"},"exitCode":{"type":["integer","null"],"format":"int32"},"logs":nullable_string(),"errorMessage":nullable_string()}}));
    schemas.insert("AutomationActionRunStreamItem".into(), json!({"type":"object","properties":{"runId":nullable_uuid(),"status":nullable_string(),"stream":nullable_string(),"progressMessage":nullable_string(),"errorMessage":nullable_string()}}));
    schemas.insert("AutomationActionRunsView".into(), json!({"type":"object","required":["runs"],"properties":{"runs":{"type":"array","items":{"$ref":"#/components/schemas/AutomationActionRunView"}}}}));
    schemas.insert("AutomationActionRunLogsView".into(), json!({"type":"object","required":["runId","logs"],"properties":{"runId":uuid(),"logs":string()}}));

    schemas.insert("BuildArgSpec".into(), json!({"type":"object","required":["name","value"],"properties":{"name":string(),"value":nullable_string()}}));
    schemas.insert("BuildSecretSpec".into(), json!({"type":"object","required":["id","secretId"],"properties":{"id":string(),"secretId":uuid()}}));
    schemas.insert(
        "BuildProjectBuilderKind".into(),
        json!({"type":"string","enum":["Platform","BuildAgentPool"]}),
    );
    schemas.insert("BuildProjectInput".into(), json!({"type":"object","required":["name","enabled","gitRepositoryId","registryId","imageRepository"],"properties":{"name":string(),"description":nullable_string(),"enabled":{"type":"boolean"},"gitRepositoryId":uuid(),"branch":nullable_string(),"contextPath":nullable_string(),"dockerfilePath":nullable_string(),"target":nullable_string(),"buildArgs":{"type":"array","items":{"$ref":"#/components/schemas/BuildArgSpec"}},"buildSecrets":{"type":"array","items":{"$ref":"#/components/schemas/BuildSecretSpec"}},"platformId":nullable_uuid(),"registryId":uuid(),"imageRepository":string(),"tagTemplates":string_array(),"webhook":{"type":["object","null"]},"timeoutSeconds":{"type":["integer","null"],"format":"int32"},"retentionRunCount":{"type":["integer","null"],"format":"int32"},"tagIds":uuid_array(),"builderKind":{"$ref":"#/components/schemas/BuildProjectBuilderKind"},"buildAgentPoolId":nullable_uuid()}}));
    schemas.insert("BuildProjectView".into(), json!({"type":"object","required":["id","name","normalizedName","enabled","gitRepositoryId","branch","contextPath","dockerfilePath","buildArgs","buildSecrets","builderKind","registryId","imageRepository","tagTemplates","timeoutSeconds","retentionRunCount","controlState","createdByActorId","createdAt","updatedAt","rowVersion"],"properties":{"id":uuid(),"name":string(),"normalizedName":string(),"description":nullable_string(),"enabled":{"type":"boolean"},"gitRepositoryId":uuid(),"branch":string(),"contextPath":string(),"dockerfilePath":string(),"target":nullable_string(),"buildArgs":{"type":"array","items":{"$ref":"#/components/schemas/BuildArgSpec"}},"buildSecrets":{"type":"array","items":{"$ref":"#/components/schemas/BuildSecretSpec"}},"builderKind":{"$ref":"#/components/schemas/BuildProjectBuilderKind"},"platformId":nullable_uuid(),"buildAgentPoolId":nullable_uuid(),"registryId":uuid(),"imageRepository":string(),"tagTemplates":string_array(),"webhook":{"type":["object","null"]},"timeoutSeconds":{"type":"integer","format":"int32"},"retentionRunCount":{"type":"integer","format":"int32"},"currentRunId":nullable_uuid(),"controlState":string(),"controlStartedAt":{"type":["integer","null"],"format":"int64"},"createdByActorId":uuid(),"createdAt":{"type":"string","format":"date-time"},"updatedAt":{"type":"string","format":"date-time"},"archivedAt":{"type":["string","null"],"format":"date-time"},"rowVersion":{"type":"integer","format":"int64"}}}));
    schemas.insert("BuildProjectsView".into(), json!({"type":"object","required":["projects"],"properties":{"projects":{"type":"array","items":{"$ref":"#/components/schemas/BuildProjectView"}}}}));
    schemas.insert("QueueBuildRunInput".into(), json!({"type":"object","properties":{"trigger":string(),"triggerSourceId":nullable_uuid()}}));
    schemas.insert("BuildRunView".into(), json!({"type":"object","required":["id","buildProjectId","projectNameSnapshot","gitRepositoryId","branch","contextPath","dockerfilePath","imageRepository","imageReferences","trigger","status","timeoutSeconds","queuedAt","triggeredByActorId"],"properties":{"id":uuid(),"buildProjectId":uuid(),"projectNameSnapshot":string(),"gitRepositoryId":uuid(),"branch":string(),"resolvedCommitSha":nullable_string(),"contextPath":string(),"dockerfilePath":string(),"target":nullable_string(),"imageRepository":string(),"imageReferences":string_array(),"trigger":string(),"status":string(),"imageDigest":nullable_string(),"timeoutSeconds":{"type":"integer","format":"int32"},"queuedAt":{"type":"string","format":"date-time"},"startedAt":{"type":["string","null"],"format":"date-time"},"completedAt":{"type":["string","null"],"format":"date-time"},"exitCode":{"type":["integer","null"],"format":"int32"},"errorCode":nullable_string(),"errorMessage":nullable_string(),"triggeredByActorId":uuid()}}));
    schemas.insert("BuildRunsView".into(), json!({"type":"object","required":["runs"],"properties":{"runs":{"type":"array","items":{"$ref":"#/components/schemas/BuildRunView"}}}}));
    schemas.insert("BuildLogsView".into(), json!({"type":"object","required":["runId","logs"],"properties":{"runId":uuid(),"logs":{"type":"array","items":{"type":"object","properties":{"stream":string(),"message":string()}}}}}));
    schemas.insert("BuildAgentPoolInput".into(), json!({"type":"object","required":["name","enabled","providerSpec"],"properties":{"name":string(),"description":nullable_string(),"enabled":{"type":"boolean"},"providerSpec":{"type":"object"},"maxActiveBuilders":{"type":["integer","null"],"format":"int32"},"queueTimeoutSeconds":{"type":["integer","null"],"format":"int32"},"provisioningTimeoutSeconds":{"type":["integer","null"],"format":"int32"},"registrationTimeoutSeconds":{"type":["integer","null"],"format":"int32"},"heartbeatTimeoutSeconds":{"type":["integer","null"],"format":"int32"},"cleanupTimeoutSeconds":{"type":["integer","null"],"format":"int32"},"maximumInstanceLifetimeSeconds":{"type":["integer","null"],"format":"int32"},"failureRetentionMinutes":{"type":["integer","null"],"format":"int32"},"tagIds":uuid_array()}}));
    schemas.insert("BuildAgentPoolView".into(), json!({"allOf":[{"$ref":"#/components/schemas/BuildAgentPoolInput"},{"type":"object","required":["id","normalizedName","provider","lastValidationStatus","controlState","createdByActorId","createdAt","updatedAt","rowVersion"],"properties":{"id":uuid(),"normalizedName":string(),"provider":string(),"lastValidationStatus":string(),"lastValidationMessage":nullable_string(),"lastValidatedAt":nullable_date_time(),"controlState":string(),"controlTriggeredBy":nullable_uuid(),"controlStartedAt":{"type":["integer","null"],"format":"int64"},"createdByActorId":uuid(),"createdAt":date_time(),"updatedAt":date_time(),"archivedAt":nullable_date_time(),"rowVersion":{"type":"integer","format":"int64"}}}]}));
    schemas.insert("BuildAgentPoolsView".into(), json!({"type":"object","required":["buildAgentPools"],"properties":{"buildAgentPools":{"type":"array","items":{"$ref":"#/components/schemas/BuildAgentPoolView"}}}}));
    schemas.insert("AlertChannelInput".into(), json!({"type":"object","required":["name","alertDestination","url","isActive"],"properties":{"name":string(),"alertDestination":string(),"url":string(),"isActive":{"type":"boolean"}}}));
    schemas.insert("VerifyAlertChannelInput".into(), json!({"type":"object","required":["name","alertDestination","url"],"properties":{"name":string(),"alertDestination":string(),"url":string()}}));
    schemas.insert("AlertChannelView".into(), json!({"allOf":[{"$ref":"#/components/schemas/AlertChannelInput"},{"type":"object","required":["id","createdByActorId","createdAt"],"properties":{"id":uuid(),"createdByActorId":uuid(),"createdAt":date_time()}}]}));
    schemas.insert("AlertChannelsView".into(), json!({"type":"object","required":["channels"],"properties":{"channels":{"type":"array","items":{"$ref":"#/components/schemas/AlertChannelView"}}}}));
    schemas.insert("AlertRuleInput".into(), json!({"type":"object","required":["name","type","severity","status","channelIds","limitedTo","quietHours"],"properties":{"name":string(),"description":nullable_string(),"type":string(),"severity":string(),"cooldownSeconds":{"type":["integer","null"],"format":"int32"},"requiredMatches":{"type":["integer","null"],"format":"int32"},"threshold":{"type":["number","null"],"format":"double"},"status":string(),"channelIds":uuid_array(),"limitedTo":{"type":"array","items":{"type":"object"}},"quietHours":{"type":"array","items":{"type":"object"}}}}));
    schemas.insert(
        "CreateAlertRuleInput".into(),
        json!({"$ref":"#/components/schemas/AlertRuleInput"}),
    );
    schemas.insert(
        "PatchAlertRuleInput".into(),
        json!({"$ref":"#/components/schemas/AlertRuleInput"}),
    );
    schemas.insert("AlertRuleView".into(), json!({"allOf":[{"$ref":"#/components/schemas/AlertRuleInput"},{"type":"object","required":["id","createdByActorId","createdAt"],"properties":{"id":uuid(),"createdByActorId":uuid(),"createdAt":date_time()}}]}));
    schemas.insert("AlertRulesView".into(), json!({"type":"object","required":["alertRules"],"properties":{"alertRules":{"type":"array","items":{"$ref":"#/components/schemas/AlertRuleView"}}}}));
    schemas.insert("AlertEventView".into(), json!({"type":"object","required":["id","alertRuleId","type","severity","status","message","info","resourceName","resourceType","createdAt","updatedAt"],"properties":{"id":uuid(),"alertRuleId":uuid(),"type":string(),"severity":string(),"status":string(),"message":string(),"info":{"type":"object"},"resourceId":nullable_uuid(),"resourceName":string(),"resourceType":string(),"acknowledgedByActorId":nullable_uuid(),"acknowledgedAt":nullable_date_time(),"resolvedByActorId":nullable_uuid(),"resolvedAt":nullable_date_time(),"resolutionNote":nullable_string(),"createdAt":date_time(),"updatedAt":date_time()}}));
    schemas.insert("AlertEventsView".into(), json!({"type":"object","required":["pagedResult"],"properties":{"pagedResult":{"type":"object","required":["items","totalCount","page","pageSize"],"properties":{"items":{"type":"array","items":{"$ref":"#/components/schemas/AlertEventView"}},"totalCount":{"type":"integer","format":"int64"},"page":{"type":"integer","format":"int32"},"pageSize":{"type":"integer","format":"int32"}}}}}));
    schemas.insert(
        "AlertIdsInput".into(),
        json!({"type":"object","required":["ids"],"properties":{"ids":uuid_array()}}),
    );
    for name in [
        "DeleteAlertChannelsInput",
        "DeleteAlertRulesInput",
        "AcknowledgeAlertEventsInput",
    ] {
        schemas.insert(
            name.into(),
            json!({"$ref":"#/components/schemas/AlertIdsInput"}),
        );
    }
    schemas.insert("ResolveAlertEventsInput".into(), json!({"type":"object","required":["ids"],"properties":{"ids":uuid_array(),"resolutionNote":nullable_string()}}));
    schemas.insert("UnresolvedAlertsCountView".into(), json!({"type":"object","required":["count"],"properties":{"count":{"type":"integer","format":"int64"}}}));

    let backup_repository_spec = json!({"type":"object","required":["$type"],"properties":{"$type":string()},"additionalProperties":true});
    let backup_source_spec = json!({"type":"object","required":["$type"],"properties":{"$type":string()},"additionalProperties":true});
    schemas.insert("BackupRepositorySpec".into(), backup_repository_spec);
    schemas.insert("BackupSourceSpec".into(), backup_source_spec);
    schemas.insert("BackupRepositoryInput".into(), json!({"type":"object","required":["name","description","spec","passwordSecretId"],"properties":{"name":string(),"description":nullable_string(),"spec":{"$ref":"#/components/schemas/BackupRepositorySpec"},"passwordSecretId":uuid()}}));
    schemas.insert("BackupRepositoryView".into(), json!({"type":"object","required":["id","name","normalizedName","description","type","spec","passwordSecretId","status","controlState","currentRunId","controlStartedAt","lastPrunedAt","lastCheckedAt","createdByActorId","createdAt","updatedAt","archivedAt","rowVersion"],"properties":{"id":uuid(),"name":string(),"normalizedName":string(),"description":nullable_string(),"type":string(),"spec":{"$ref":"#/components/schemas/BackupRepositorySpec"},"passwordSecretId":uuid(),"status":string(),"controlState":string(),"currentRunId":nullable_uuid(),"controlStartedAt":{"type":["integer","null"],"format":"int64"},"lastPrunedAt":nullable_date_time(),"lastCheckedAt":nullable_date_time(),"createdByActorId":uuid(),"createdAt":date_time(),"updatedAt":date_time(),"archivedAt":nullable_date_time(),"rowVersion":{"type":"integer","format":"int64"}}}));
    schemas.insert("BackupRepositoriesView".into(), json!({"type":"object","required":["repositories","capabilities"],"properties":{"repositories":{"type":"array","items":{"$ref":"#/components/schemas/BackupRepositoryView"}},"capabilities":{"$ref":"#/components/schemas/ResourceCapabilities"}}}));
    schemas.insert("ValidateBackupRepositoryInput".into(), json!({"type":"object","required":["location","platformId"],"properties":{"location":string(),"platformId":nullable_uuid()}}));
    schemas.insert("BackupRepositoryValidationView".into(), json!({"type":"object","required":["id","backupRepositoryId","location","platformId","status","lastValidatedAt","lastErrorCode","lastErrorMessage"],"properties":{"id":uuid(),"backupRepositoryId":uuid(),"location":string(),"platformId":nullable_uuid(),"status":string(),"lastValidatedAt":date_time(),"lastErrorCode":nullable_string(),"lastErrorMessage":nullable_string()}}));
    schemas.insert("BackupPolicyInput".into(), json!({"type":"object","required":["name","description","source","backupRepositoryId"],"properties":{"name":string(),"description":nullable_string(),"source":{"$ref":"#/components/schemas/BackupSourceSpec"},"backupRepositoryId":uuid(),"enabled":{"type":"boolean","default":true},"cron":nullable_string(),"timeZone":nullable_string(),"webhook":{"type":["object","null"]},"keepLastSuccessful":{"type":["integer","null"],"format":"int32"},"timeoutSeconds":{"type":["integer","null"],"format":"int32"},"alertOnFailure":{"type":"boolean","default":true},"runAsActorId":nullable_uuid(),"tagIds":{"type":["array","null"],"items":uuid()}}}));
    schemas.insert("BackupPolicyView".into(), json!({"type":"object","required":["id","name","normalizedName","description","source","backupRepositoryId","enabled","cron","timeZone","webhook","keepLastSuccessful","timeoutSeconds","alertOnFailure","runAsActorId","controlState","currentRunId","lastScheduledRunAt","firstSuccessfulRunAt","createdByActorId","createdAt","updatedAt","archivedAt","rowVersion"],"properties":{"id":uuid(),"name":string(),"normalizedName":string(),"description":nullable_string(),"source":{"$ref":"#/components/schemas/BackupSourceSpec"},"backupRepositoryId":uuid(),"enabled":{"type":"boolean"},"cron":nullable_string(),"timeZone":nullable_string(),"webhook":{"type":["object","null"]},"keepLastSuccessful":{"type":"integer","format":"int32"},"timeoutSeconds":{"type":"integer","format":"int32"},"alertOnFailure":{"type":"boolean"},"runAsActorId":uuid(),"controlState":string(),"currentRunId":nullable_uuid(),"lastScheduledRunAt":nullable_date_time(),"firstSuccessfulRunAt":nullable_date_time(),"createdByActorId":uuid(),"createdAt":date_time(),"updatedAt":date_time(),"archivedAt":nullable_date_time(),"rowVersion":{"type":"integer","format":"int64"}}}));
    schemas.insert("BackupPoliciesView".into(), json!({"type":"object","required":["policies","capabilities"],"properties":{"policies":{"type":"array","items":{"$ref":"#/components/schemas/BackupPolicyView"}},"capabilities":{"$ref":"#/components/schemas/ResourceCapabilities"}}}));
    schemas.insert("QueueBackupRunInput".into(), json!({"type":"object","properties":{"trigger":string(),"triggerSourceId":nullable_uuid()}}));
    schemas.insert("BackupRunItemView".into(), json!({"type":"object","required":["id","backupRunId","platformId","volumeName","status"],"properties":{"id":uuid(),"backupRunId":uuid(),"platformId":uuid(),"volumeName":string(),"dockerNodeId":nullable_string(),"nodeHostname":nullable_string(),"status":string(),"resticSnapshotId":nullable_string(),"parentSnapshotId":nullable_string(),"filesProcessed":{"type":["integer","null"],"format":"int64"},"bytesProcessed":{"type":["integer","null"],"format":"int64"},"bytesAdded":{"type":["integer","null"],"format":"int64"},"startedAt":nullable_date_time(),"completedAt":nullable_date_time(),"exitCode":{"type":["integer","null"],"format":"int32"},"errorCode":nullable_string(),"errorMessage":nullable_string()}}));
    schemas.insert("BackupRunView".into(), json!({"type":"object","required":["id","backupPolicyId","policyNameSnapshot","backupRepositoryId","repositoryTypeSnapshot","sourceSnapshot","trigger","status","snapshotAvailability","warnings","queuedAt","triggeredByActorId","items"],"properties":{"id":uuid(),"backupPolicyId":uuid(),"policyNameSnapshot":string(),"backupRepositoryId":uuid(),"repositoryTypeSnapshot":string(),"sourceSnapshot":{"$ref":"#/components/schemas/BackupSourceSpec"},"trigger":string(),"status":string(),"snapshotAvailability":string(),"resticSnapshotId":nullable_string(),"parentSnapshotId":nullable_string(),"filesProcessed":{"type":["integer","null"],"format":"int64"},"bytesProcessed":{"type":["integer","null"],"format":"int64"},"bytesAdded":{"type":["integer","null"],"format":"int64"},"warnings":string_array(),"queuedAt":date_time(),"startedAt":nullable_date_time(),"completedAt":nullable_date_time(),"exitCode":{"type":["integer","null"],"format":"int32"},"errorCode":nullable_string(),"errorMessage":nullable_string(),"triggeredByActorId":uuid(),"items":{"type":"array","items":{"$ref":"#/components/schemas/BackupRunItemView"}}}}));
    schemas.insert("BackupRunsView".into(), json!({"type":"object","required":["runs"],"properties":{"runs":{"type":"array","items":{"$ref":"#/components/schemas/BackupRunView"}}}}));
    schemas.insert("BackupLogsView".into(), json!({"type":"object","required":["runId","logs"],"properties":{"runId":uuid(),"logs":string()}}));
    schemas.insert("RestoreVolumeInput".into(), json!({"type":"object","required":["targetPlatformId","targetVolumeName","overwriteExisting"],"properties":{"targetPlatformId":uuid(),"targetVolumeName":string(),"overwriteExisting":{"type":"boolean"},"targetDockerNodeId":nullable_string(),"sourceBackupRunItemId":nullable_uuid()}}));
    schemas.insert("BackupRestoreRunView".into(), json!({"type":"object","required":["id","backupRunId","backupRepositoryId","targetPlatformId","targetVolumeName","overwriteExisting","status","queuedAt","triggeredByActorId"],"properties":{"id":uuid(),"backupRunId":uuid(),"backupRepositoryId":uuid(),"sourceBackupRunItemId":nullable_uuid(),"targetPlatformId":uuid(),"targetDockerNodeId":nullable_string(),"targetVolumeName":string(),"overwriteExisting":{"type":"boolean"},"status":string(),"queuedAt":date_time(),"startedAt":nullable_date_time(),"completedAt":nullable_date_time(),"exitCode":{"type":["integer","null"],"format":"int32"},"errorCode":nullable_string(),"errorMessage":nullable_string(),"triggeredByActorId":uuid()}}));
    schemas.insert("BackupRestoreRunsView".into(), json!({"type":"object","required":["runs"],"properties":{"runs":{"type":"array","items":{"$ref":"#/components/schemas/BackupRestoreRunView"}}}}));

    schemas.insert(
        "CreateNetworkInput".into(),
        json!({"type":"object","required":["platformId","name","driver","scope"],"properties":{"platformId":uuid(),"name":string(),"driver":string(),"scope":string(),"internal":{"type":["boolean","null"]},"attachable":{"type":["boolean","null"]},"ingress":{"type":["boolean","null"]},"enableIPv6":{"type":["boolean","null"]},"enableIPv4":{"type":["boolean","null"]},"configOnly":{"type":["boolean","null"]},"ipam":{"type":["object","null"]},"configFrom":{"type":["object","null"]},"labels":string_map(),"options":string_map()}}),
    );
    schemas.insert(
        "CreateNetworkView".into(),
        json!({"type":"object","required":["id"],"properties":{"id":string()}}),
    );
    schemas.insert(
        "DeleteNetworksInput".into(),
        platform_ids_input_schema("ids"),
    );
    schemas.insert(
        "CreateVolumeInput".into(),
        json!({"type":"object","required":["platformId","name","driver"],"properties":{"platformId":uuid(),"name":string(),"driver":string(),"labels":string_map(),"options":string_map()}}),
    );
    schemas.insert(
        "DeleteVolumesInput".into(),
        json!({"type":"object","required":["platformId","names"],"properties":{"platformId":uuid(),"names":string_array(),"force":{"type":["boolean","null"]}}}),
    );

    schemas.extend(binding_schemas());
    schemas
}

fn catalog_input_schema(registry: bool) -> Value {
    if registry {
        json!({"type":"object","required":["name","registryHost","status","configuration"],"properties":{"name":string(),"description":nullable_string(),"registryHost":string(),"status":string(),"configuration":{"type":"object"},"tagIds":uuid_array()}})
    } else {
        json!({"type":"object","required":["name","url","defaultBranch"],"properties":{"name":string(),"description":nullable_string(),"url":string(),"defaultBranch":string(),"gitAccountId":nullable_uuid(),"syncMode":{"$ref":"#/components/schemas/GitRepositorySyncMode"},"syncIntervalMinutes":{"type":["integer","null"],"format":"int32"},"webhook":{"oneOf":[{"type":"null"},{"$ref":"#/components/schemas/RepoWebhookConfig"}]},"onClone":{"oneOf":[{"type":"null"},{"$ref":"#/components/schemas/RepoCommand"}]},"onPull":{"oneOf":[{"type":"null"},{"$ref":"#/components/schemas/RepoCommand"}]},"tagIds":uuid_array()}})
    }
}

fn catalog_patch_schema(registry: bool) -> Value {
    if registry {
        json!({"type":"object","properties":{"registryHost":string(),"status":string(),"configuration":{"type":"object"}}})
    } else {
        json!({"type":"object","properties":{"url":string(),"defaultBranch":string(),"gitAccountId":nullable_uuid(),"syncMode":{"$ref":"#/components/schemas/GitRepositorySyncMode"},"syncIntervalMinutes":{"type":["integer","null"],"format":"int32"},"webhook":{"oneOf":[{"type":"null"},{"$ref":"#/components/schemas/RepoWebhookConfig"}]},"onClone":{"oneOf":[{"type":"null"},{"$ref":"#/components/schemas/RepoCommand"}]},"onPull":{"oneOf":[{"type":"null"},{"$ref":"#/components/schemas/RepoCommand"}]}}})
    }
}

fn ids_input_schema() -> Value {
    json!({"type":"object","required":["ids"],"properties":{"ids":uuid_array()}})
}

fn platform_ids_input_schema(property: &str) -> Value {
    json!({"type":"object","required":["platformId",property],"properties":{"platformId":uuid(),property:string_array()}})
}

fn binding_schemas() -> Map<String, Value> {
    let mut schemas = Map::new();
    let binding = json!({"type":"object","required":["id","name","kind","scope","isInherited"],"properties":{"id":uuid(),"name":string(),"kind":{"type":"string","enum":["Variable","Secret"]},"scope":{"$ref":"#/components/schemas/ResourceBindingScope"},"resourceId":nullable_uuid(),"value":nullable_string(),"secretId":nullable_uuid(),"secretDeliveryMode":{"type":["string","null"]},"targetPath":nullable_string(),"isInherited":{"type":"boolean"}}});
    schemas.insert("ResourceBindingView".into(), binding);
    schemas.insert(
        "ResourceBindingsView".into(),
        json!({"type":"object","required":["entries","effectiveEntries"],"properties":{"entries":{"type":"array","items":{"$ref":"#/components/schemas/ResourceBindingView"}},"effectiveEntries":{"type":"array","items":{"$ref":"#/components/schemas/ResourceBindingView"}},"capabilities":{"$ref":"#/components/schemas/ResourceCapabilities"}}}),
    );
    schemas.insert("ResourceBindingInput".into(), binding_input_schema(false));
    schemas.insert(
        "UpdateResourceBindingInput".into(),
        binding_input_schema(true),
    );
    schemas.insert(
        "SecretDefinitionView".into(),
        json!({"type":"object","required":["id","name","providerType","createdAt"],"properties":{"id":uuid(),"name":string(),"providerType":string(),"providerId":nullable_uuid(),"externalPath":nullable_string(),"externalKey":nullable_string(),"externalVersion":{"type":["integer","null"]},"createdAt":{"type":"string","format":"date-time"}}}),
    );
    schemas.insert(
        "SecretDefinitionsView".into(),
        collection_view("secrets", "SecretDefinitionView", "ResourceCapabilities"),
    );
    schemas.insert(
        "CreateInternalSecretInput".into(),
        json!({"type":"object","required":["name","value"],"properties":{"name":string(),"value":{"type":"string","writeOnly":true}}}),
    );
    schemas.insert(
        "CreateExternalSecretInput".into(),
        external_secret_schema(false),
    );
    schemas.insert(
        "UpdateExternalSecretInput".into(),
        external_secret_schema(true),
    );
    schemas.insert(
        "SecretProviderView".into(),
        json!({"type":"object","required":["id","name","providerType","address","mountPath","createdAt"],"properties":{"id":uuid(),"name":string(),"providerType":string(),"address":string(),"mountPath":string(),"createdAt":{"type":"string","format":"date-time"}}}),
    );
    schemas.insert(
        "SecretProvidersView".into(),
        json!({"type":"object","required":["providers"],"properties":{"providers":{"type":"array","items":{"$ref":"#/components/schemas/SecretProviderView"}}}}),
    );
    schemas.insert(
        "CreateVaultKvV2SecretProviderInput".into(),
        provider_schema(false),
    );
    schemas.insert(
        "UpdateVaultKvV2SecretProviderInput".into(),
        provider_schema(true),
    );
    schemas
}

fn binding_input_schema(update: bool) -> Value {
    let mut required = vec!["name", "kind"];
    if update {
        required.insert(0, "id");
    }
    json!({"type":"object","required":required,"properties":{"id":uuid(),"name":string(),"kind":string(),"value":nullable_string(),"secretId":nullable_uuid(),"secretDeliveryMode":{"type":["string","null"]},"targetPath":nullable_string()}})
}

fn external_secret_schema(update: bool) -> Value {
    let required = if update {
        Vec::<&str>::new()
    } else {
        vec!["name", "providerId", "externalPath", "externalKey"]
    };
    json!({"type":"object","required":required,"properties":{"name":string(),"providerId":uuid(),"externalPath":string(),"externalKey":string(),"externalVersion":{"type":["integer","null"]}}})
}

fn provider_schema(update: bool) -> Value {
    let required = if update {
        Vec::<&str>::new()
    } else {
        vec!["name", "address", "mountPath", "token"]
    };
    json!({"type":"object","required":required,"properties":{"name":string(),"address":string(),"mountPath":string(),"token":{"type":"string","writeOnly":true}}})
}

fn collection_view(property: &str, item_schema: &str, capabilities_schema: &str) -> Value {
    let properties = Map::from_iter([
        (
            property.to_owned(),
            json!({ "type": "array", "items": { "$ref": format!("#/components/schemas/{item_schema}") } }),
        ),
        (
            "capabilities".to_owned(),
            json!({ "$ref": format!("#/components/schemas/{capabilities_schema}") }),
        ),
    ]);
    json!({
        "type": "object", "required": [property, "capabilities"], "additionalProperties": false,
        "properties": properties
    })
}

fn platform_capabilities() -> Value {
    json!({
        "type": "object", "additionalProperties": false,
        "required": ["canRead", "canWrite", "canExecute", "canViewLogs", "canInspect", "canOpenTerminal", "canPull", "canManageNodeAgents"],
        "properties": {
            "canRead": {"type":"boolean"}, "canWrite": {"type":"boolean"},
            "canExecute": {"type":"boolean"}, "canViewLogs": {"type":"boolean"},
            "canInspect": {"type":"boolean"}, "canOpenTerminal": {"type":"boolean"},
            "canPull": {"type":"boolean"}, "canManageNodeAgents": {"type":"boolean"}
        }
    })
}

fn image_capabilities() -> Value {
    json!({
        "type":"object", "additionalProperties":false,
        "required":["canRead","canWrite","canExecute","canInspect","canPull"],
        "properties":{
            "canRead":{"type":"boolean"}, "canWrite":{"type":"boolean"},
            "canExecute":{"type":"boolean"}, "canInspect":{"type":"boolean"},
            "canPull":{"type":"boolean"}
        }
    })
}

fn network_capabilities() -> Value {
    json!({
        "type":"object", "additionalProperties":false,
        "required":["canRead","canWrite","canExecute","canInspect"],
        "properties":{
            "canRead":{"type":"boolean"}, "canWrite":{"type":"boolean"},
            "canExecute":{"type":"boolean"}, "canInspect":{"type":"boolean"}
        }
    })
}

fn volume_capabilities() -> Value {
    json!({
        "type":"object", "additionalProperties":false,
        "required":["canRead","canWrite","canExecute","canInspect","canBrowse","canDownload"],
        "properties":{
            "canRead":{"type":"boolean"}, "canWrite":{"type":"boolean"},
            "canExecute":{"type":"boolean"}, "canInspect":{"type":"boolean"},
            "canBrowse":{"type":"boolean"}, "canDownload":{"type":"boolean"}
        }
    })
}

fn nullable(schema: Value) -> Value {
    json!({ "oneOf": [{ "type": "null" }, schema] })
}

fn string_array() -> Value {
    json!({"type":"array", "items":{"type":"string"}})
}

fn string_map() -> Value {
    json!({"type":"object", "additionalProperties":{"type":"string"}})
}

fn platform_view() -> Value {
    json!({
        "type":"object", "additionalProperties":true,
        "required":["id","name","address","networkCount","volumeCount","imageCount","cpuCount","memTotal","type","status","connectorType","deploymentCount","stackCount","deploymentStatusCounts","stackStatusCounts","swarmServiceStatusCounts","pruneHistoricalSwarmTaskContainers"],
        "properties":{
            "id":uuid(), "name":string(), "description":nullable_string(), "address":string(),
            "networkCount":integer(), "volumeCount":integer(), "imageCount":integer(),
            "cpuCount":integer(), "memTotal":integer(), "agentVersion":nullable_string(),
            "serverVersion":nullable_string(), "type":string(), "status":string(),
            "connectorType":string(), "deploymentCount":integer(), "stackCount":integer(),
            "deploymentStatusCounts":{"type":"object"}, "stackStatusCounts":{"type":"object"},
            "swarmServiceStatusCounts":{"type":"object"}, "stats":{"type":["array","null"]},
            "platformDescriptor":{}, "clusterId":nullable_string(),
            "pruneHistoricalSwarmTaskContainers":{"type":"boolean"},
            "capabilities":nullable(json!({"$ref":"#/components/schemas/PlatformCapabilities"}))
        }
    })
}

fn container_view() -> Value {
    json!({
        "type":"object", "additionalProperties":true,
        "required":["id","platformId","containerId","name","dockerImageId","created","state","controlState","updated","isSystem","hasCitadelOwnershipLabels","isSwarmTask","ports"],
        "properties":{
            "id":uuid(), "platformId":uuid(), "containerId":string(), "name":string(),
            "dockerImageId":string(), "created":integer(), "state":string(), "controlState":string(),
            "updated":integer(), "stack":nullable_string(), "isSystem":{"type":"boolean"},
            "systemRole":nullable_string(), "hasCitadelOwnershipLabels":{"type":"boolean"},
            "isSwarmTask":{"type":"boolean"}, "dockerNodeId":nullable_string(),
            "nodeHostname":nullable_string(), "projectionObservedAt":{"type":["integer","null"]},
            "projectionStaleSince":{"type":["integer","null"]}, "projectionStaleReason":nullable_string(),
            "lastStats":nullable(json!({"type":"object"})), "ports":{},
            "deploymentId":nullable_uuid(), "stackId":nullable_uuid(),
            "capabilities":nullable(json!({"$ref":"#/components/schemas/PlatformCapabilities"}))
        }
    })
}

fn image_view() -> Value {
    json!({
        "type":"object", "additionalProperties":true,
        "required":["id","tags","name","dockerImageId","size","isInUse","platformId","createdAt","controlState"],
        "properties":{
            "id":uuid(), "tags":string_array(), "name":string(), "dockerImageId":string(),
            "size":{"type":"number","format":"double"}, "isInUse":{"type":"boolean"},
            "platformId":uuid(), "createdAt":{"type":"string","format":"date-time"},
            "controlState":string(), "updatedAt":nullable_date_time(), "registryId":nullable_uuid(),
            "repoDigests":{"type":["array","null"],"items":{"type":"string"}},
            "contentIdentity":nullable_string(), "dockerNodeId":nullable_string(),
            "nodeHostname":nullable_string(), "isStale":{"type":"boolean"},
            "staleReason":nullable_string(),
            "capabilities":nullable(json!({"$ref":"#/components/schemas/ImageCapabilities"}))
        }
    })
}

fn network_view() -> Value {
    json!({
        "type":"object", "additionalProperties":true,
        "required":["name","id","created","driver","scope","enableIPv4","enableIPv6","internal","attachable","ingress","configOnly","options","labels","containers","peers","isSystem"],
        "properties":{
            "name":string(), "id":string(), "created":string(), "driver":string(), "scope":string(),
            "enableIPv4":{"type":"boolean"}, "enableIPv6":{"type":"boolean"},
            "internal":{"type":"boolean"}, "attachable":{"type":"boolean"},
            "ingress":{"type":"boolean"}, "configOnly":{"type":"boolean"},
            "inUse":{"type":"boolean"}, "configFrom":nullable_string(), "ipam":{},
            "options":string_map(), "labels":string_map(), "isSystem":{"type":"boolean"},
            "containers":{"type":"object","additionalProperties":{}}, "peers":{"type":"array","items":{}},
            "dockerNodeId":nullable_string(), "nodeHostname":nullable_string(),
            "isStale":{"type":"boolean"}, "staleReason":nullable_string(),
            "capabilities":nullable(json!({"$ref":"#/components/schemas/NetworkCapabilities"}))
        }
    })
}

fn volume_view() -> Value {
    json!({
        "type":"object", "additionalProperties":true,
        "required":["id","name","inUse","scope","driver","mountpoint","createdAt","containers","status","labels","options"],
        "properties":{
            "id":string(), "name":string(), "inUse":{"type":"boolean"}, "scope":string(),
            "driver":string(), "mountpoint":string(), "createdAt":string(), "clusterVolume":{},
            "usageData":{}, "containers":{"type":"array"}, "status":string_map(),
            "labels":string_map(), "options":string_map(), "dockerNodeId":nullable_string(),
            "nodeHostname":nullable_string(), "isStale":{"type":"boolean"},
            "staleReason":nullable_string(),
            "capabilities":nullable(json!({"$ref":"#/components/schemas/VolumeCapabilities"}))
        }
    })
}

fn swarm_base(properties: Map<String, Value>, required: Vec<&str>) -> Value {
    json!({"type":"object", "additionalProperties":false, "required":required, "properties":properties})
}

fn swarm_common() -> Map<String, Value> {
    Map::from_iter([
        ("id".into(), string()),
        ("createdAt".into(), nullable_date_time()),
        (
            "observedAt".into(),
            json!({"type":"string","format":"date-time"}),
        ),
        ("isStale".into(), json!({"type":"boolean"})),
        (
            "capabilities".into(),
            nullable(json!({"$ref":"#/components/schemas/PlatformCapabilities"})),
        ),
    ])
}

fn swarm_node_view() -> Value {
    let mut p = swarm_common();
    p.extend(Map::from_iter([
        ("versionIndex".into(), integer()),
        ("hostname".into(), string()),
        ("role".into(), string()),
        ("isLeader".into(), json!({"type":"boolean"})),
        ("reachability".into(), string()),
        ("status".into(), string()),
        ("statusMessage".into(), nullable_string()),
        ("availability".into(), string()),
        ("engineVersion".into(), string()),
        ("operatingSystem".into(), string()),
        ("architecture".into(), string()),
        ("address".into(), string()),
        ("labels".into(), string_map()),
        ("runningTaskCount".into(), integer()),
        ("desiredTaskCount".into(), integer()),
        ("updatedAt".into(), nullable_date_time()),
    ]));
    swarm_base(
        p,
        vec![
            "id",
            "versionIndex",
            "hostname",
            "role",
            "isLeader",
            "reachability",
            "status",
            "availability",
            "engineVersion",
            "operatingSystem",
            "architecture",
            "address",
            "labels",
            "runningTaskCount",
            "desiredTaskCount",
            "observedAt",
            "isStale",
        ],
    )
}

fn swarm_service_view() -> Value {
    let mut p = swarm_common();
    p.extend(Map::from_iter([
        ("versionIndex".into(), integer()),
        ("name".into(), string()),
        ("mode".into(), string()),
        ("image".into(), string()),
        ("runningTaskCount".into(), integer()),
        ("desiredTaskCount".into(), integer()),
        ("updateState".into(), string()),
        ("updateMessage".into(), nullable_string()),
        ("ports".into(), string_array()),
        ("networkIds".into(), string_array()),
        ("secretIds".into(), string_array()),
        ("configIds".into(), string_array()),
        ("labels".into(), string_map()),
        ("ownership".into(), string()),
        ("dockerStackNamespace".into(), nullable_string()),
        ("ownershipDiagnostic".into(), nullable_string()),
        ("stackId".into(), nullable_uuid()),
        ("swarmServiceId".into(), nullable_uuid()),
        ("updatedAt".into(), nullable_date_time()),
    ]));
    swarm_base(
        p,
        vec![
            "id",
            "versionIndex",
            "name",
            "mode",
            "image",
            "runningTaskCount",
            "desiredTaskCount",
            "updateState",
            "ports",
            "networkIds",
            "secretIds",
            "configIds",
            "labels",
            "ownership",
            "observedAt",
            "isStale",
        ],
    )
}

fn swarm_task_view() -> Value {
    let mut p = swarm_common();
    p.extend(Map::from_iter([
        ("versionIndex".into(), integer()),
        ("name".into(), string()),
        ("serviceId".into(), string()),
        ("serviceName".into(), string()),
        ("slot".into(), json!({"type":["integer","null"]})),
        ("nodeId".into(), string()),
        ("nodeHostname".into(), string()),
        ("desiredState".into(), string()),
        ("state".into(), string()),
        ("statusMessage".into(), nullable_string()),
        ("error".into(), nullable_string()),
        ("image".into(), string()),
        ("ports".into(), string_array()),
        ("statusTimestamp".into(), nullable_date_time()),
        ("updatedAt".into(), nullable_date_time()),
    ]));
    swarm_base(
        p,
        vec![
            "id",
            "versionIndex",
            "name",
            "serviceId",
            "serviceName",
            "nodeId",
            "nodeHostname",
            "desiredState",
            "state",
            "image",
            "ports",
            "observedAt",
            "isStale",
        ],
    )
}

fn swarm_network_view() -> Value {
    let mut p = swarm_common();
    p.extend(Map::from_iter([
        ("name".into(), string()),
        ("scope".into(), string()),
        ("driver".into(), string()),
        ("isAttachable".into(), json!({"type":"boolean"})),
        ("isInternal".into(), json!({"type":"boolean"})),
        ("isIngress".into(), json!({"type":"boolean"})),
        ("isEncrypted".into(), json!({"type":"boolean"})),
        ("enableIPv6".into(), json!({"type":"boolean"})),
        ("subnets".into(), string_array()),
        ("serviceNames".into(), string_array()),
        ("labels".into(), string_map()),
    ]));
    swarm_base(
        p,
        vec![
            "id",
            "name",
            "scope",
            "driver",
            "isAttachable",
            "isInternal",
            "isIngress",
            "isEncrypted",
            "enableIPv6",
            "subnets",
            "serviceNames",
            "labels",
            "observedAt",
            "isStale",
        ],
    )
}

fn swarm_config_view() -> Value {
    swarm_named_resource_view("templatingDriver")
}

fn swarm_secret_view() -> Value {
    swarm_named_resource_view("driver")
}

fn swarm_named_resource_view(optional_property: &str) -> Value {
    let mut p = swarm_common();
    p.extend(Map::from_iter([
        ("versionIndex".into(), integer()),
        ("name".into(), string()),
        (optional_property.into(), nullable_string()),
        ("serviceNames".into(), string_array()),
        ("labels".into(), string_map()),
        ("updatedAt".into(), nullable_date_time()),
        ("inUse".into(), json!({"type":"boolean"})),
    ]));
    swarm_base(
        p,
        vec![
            "id",
            "versionIndex",
            "name",
            "serviceNames",
            "labels",
            "observedAt",
            "isStale",
            "inUse",
        ],
    )
}

fn string() -> Value {
    json!({ "type": "string" })
}

fn nullable_string() -> Value {
    json!({ "type": ["string", "null"] })
}

fn integer() -> Value {
    json!({ "type": "integer", "format": "int64" })
}

fn uuid() -> Value {
    json!({ "type": "string", "format": "uuid" })
}

fn nullable_uuid() -> Value {
    json!({ "type": ["string", "null"], "format": "uuid" })
}

fn nullable_date_time() -> Value {
    json!({ "type": ["string", "null"], "format": "date-time" })
}

fn date_time() -> Value {
    json!({ "type": "string", "format": "date-time" })
}

fn uuid_array() -> Value {
    json!({ "type": "array", "items": uuid() })
}

fn paged(item_schema: &str) -> Value {
    json!({
        "type": "object", "required": ["items", "totalCount", "page", "pageSize"], "additionalProperties": false,
        "properties": {
            "items": { "type": "array", "items": { "$ref": format!("#/components/schemas/{item_schema}") } },
            "totalCount": integer(), "page": integer(), "pageSize": integer()
        }
    })
}

fn capabilities(service_account: bool) -> Value {
    let mut properties = Map::from_iter([
        ("canRead".to_owned(), json!({ "type": "boolean" })),
        ("canWrite".to_owned(), json!({ "type": "boolean" })),
        ("canExecute".to_owned(), json!({ "type": "boolean" })),
    ]);
    let mut required = vec!["canRead", "canWrite", "canExecute"];
    if service_account {
        properties.insert("canUse".to_owned(), json!({ "type": "boolean" }));
        properties.insert(
            "canManageCredentials".to_owned(),
            json!({ "type": "boolean" }),
        );
        required.extend(["canUse", "canManageCredentials"]);
    }
    json!({
        "type": "object", "required": required, "additionalProperties": false, "properties": properties
    })
}

fn service_account_view() -> Value {
    let mut properties = Map::from_iter([
        ("id".to_owned(), uuid()),
        ("name".to_owned(), string()),
        ("description".to_owned(), nullable_string()),
        ("actorId".to_owned(), uuid()),
        ("isEnabled".to_owned(), json!({ "type": "boolean" })),
        (
            "createdAt".to_owned(),
            json!({ "type": "string", "format": "date-time" }),
        ),
        ("createdByActorId".to_owned(), uuid()),
        (
            "updatedAt".to_owned(),
            json!({ "type": "string", "format": "date-time" }),
        ),
        ("archivedAtUtc".to_owned(), nullable_date_time()),
        ("activeTokenCount".to_owned(), integer()),
        ("lastUsedAtUtc".to_owned(), nullable_date_time()),
        (
            "teams".to_owned(),
            json!({ "type": "array", "items": { "$ref": "#/components/schemas/ResourceInfo" } }),
        ),
        (
            "roles".to_owned(),
            json!({ "type": "array", "items": { "$ref": "#/components/schemas/ResourceInfo" } }),
        ),
        (
            "resourceAccesses".to_owned(),
            json!({ "type": "array", "items": { "$ref": "#/components/schemas/ServiceAccountResourceAccess" } }),
        ),
    ]);
    let required = vec![
        "id",
        "name",
        "description",
        "actorId",
        "isEnabled",
        "createdAt",
        "createdByActorId",
        "updatedAt",
        "archivedAtUtc",
        "activeTokenCount",
        "lastUsedAtUtc",
        "teams",
        "roles",
        "resourceAccesses",
    ];
    properties.insert(
        "capabilities".to_owned(),
        json!({ "oneOf": [{ "type": "null" }, { "$ref": "#/components/schemas/ServiceAccountCapabilities" }] }),
    );
    json!({
        "type": "object", "required": required, "additionalProperties": false, "properties": properties
    })
}

fn service_account_token_view(with_token: bool) -> Value {
    let mut properties = Map::from_iter([
        ("id".to_owned(), uuid()),
        ("name".to_owned(), string()),
        ("hint".to_owned(), string()),
        ("expiresAtUtc".to_owned(), nullable_date_time()),
        ("lastUsedAtUtc".to_owned(), nullable_date_time()),
        ("revokedAtUtc".to_owned(), nullable_date_time()),
        ("revokedByActorId".to_owned(), nullable_uuid()),
        ("createdByActorId".to_owned(), uuid()),
        ("createdByName".to_owned(), string()),
        (
            "createdAtUtc".to_owned(),
            json!({ "type": "string", "format": "date-time" }),
        ),
    ]);
    let mut required = vec![
        "id",
        "name",
        "hint",
        "expiresAtUtc",
        "lastUsedAtUtc",
        "revokedAtUtc",
        "revokedByActorId",
        "createdByActorId",
        "createdByName",
        "createdAtUtc",
    ];
    if with_token {
        properties.insert("token".to_owned(), string());
        required.push("token");
    }
    json!({
        "type": "object", "required": required, "additionalProperties": false, "properties": properties
    })
}

fn operation(route: &RouteContract) -> Value {
    let success = route.response_schema.map_or_else(
        || json!({ "description": "Success" }),
        |schema| {
            let item_schema = json!({"$ref": format!("#/components/schemas/{schema}")});
            let response_schema = if route.response_is_array {
                json!({"type":"array","items":item_schema})
            } else {
                item_schema
            };
            json!({
                "description": "Success",
                "content": { "application/json": { "schema": response_schema } }
            })
        },
    );
    let mut responses = Map::new();
    responses.insert(route.success_status.to_string(), success);
    for error in route.error_responses {
        responses.insert(error.status().to_string(), error_response(*error));
    }
    responses.insert(
        "default".to_owned(),
        json!({
            "description": "Request failed",
            "content": { "application/problem+json": { "schema": { "$ref": "#/components/schemas/ProblemDetails" } } }
        }),
    );
    let mut operation = json!({
        "operationId": route.operation_id,
        "summary": route.summary,
        "responses": responses,
        "security": match route.authentication {
            RouteAuthentication::Anonymous => json!([]),
            _ => json!([{ "Bearer": [] }]),
        },
        "x-citadel-principal": match route.authentication {
            RouteAuthentication::Anonymous => "anonymous",
            RouteAuthentication::Actor => "actor",
            RouteAuthentication::Human => "human",
            RouteAuthentication::Administrator => "administrator",
        }
    });
    if let Some(schema) = route.request_schema {
        operation.as_object_mut().expect("operation is an object").insert(
            "requestBody".to_owned(),
            json!({
                "required": true,
                "content": { "application/json": { "schema": { "$ref": format!("#/components/schemas/{schema}") } } }
            }),
        );
    }
    if !route.parameters.is_empty() {
        operation
            .as_object_mut()
            .expect("operation is an object")
            .insert(
                "parameters".to_owned(),
                Value::Array(route.parameters.iter().map(parameter).collect()),
            );
    }
    operation
}

fn error_response(error: ErrorResponse) -> Value {
    json!({
        "description": error.description(),
        "content": {
            "application/problem+json": {
                "schema": { "$ref": "#/components/schemas/ProblemDetails" }
            }
        }
    })
}

fn parameter(parameter: &ParameterContract) -> Value {
    json!({
        "name": parameter.name,
        "in": parameter.location.as_openapi_str(),
        "required": parameter.required,
        "schema": parameter_schema(parameter.schema),
    })
}

fn parameter_schema(schema: ParameterSchema) -> Value {
    match schema {
        ParameterSchema::String => string(),
        ParameterSchema::ArrayString => {
            json!({ "type": "array", "items": { "type": "string" } })
        }
        ParameterSchema::Boolean { default } => {
            let mut schema = Map::from_iter([("type".to_owned(), json!("boolean"))]);
            if let Some(default) = default {
                schema.insert("default".to_owned(), json!(default));
            }
            Value::Object(schema)
        }
        ParameterSchema::Uuid => uuid(),
        ParameterSchema::Integer {
            format,
            minimum,
            maximum,
            default,
        } => {
            let mut schema = Map::from_iter([
                ("type".to_owned(), json!("integer")),
                ("format".to_owned(), json!(format.as_openapi_str())),
            ]);
            if let Some(minimum) = minimum {
                schema.insert("minimum".to_owned(), json!(minimum));
            }
            if let Some(maximum) = maximum {
                schema.insert("maximum".to_owned(), json!(maximum));
            }
            if let Some(default) = default {
                schema.insert("default".to_owned(), json!(default));
            }
            Value::Object(schema)
        }
        ParameterSchema::Reference(schema) => {
            json!({ "$ref": format!("#/components/schemas/{schema}") })
        }
    }
}

fn frontend_types() -> String {
    let operations = ROUTES
        .iter()
        .map(|route| {
            format!(
                "  {}: {{ method: '{}', path: '{}' }},",
                route.operation_id,
                route.method.as_openapi_str().to_ascii_uppercase(),
                route.path
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "// @generated by `cargo xtask openapi`; do not edit.\n\
export interface HealthResponse {{ status: 'ok' }}\n\
export interface ReadinessResponse {{\n\
  status: 'ready' | 'not-ready';\n\
  database: boolean;\n\
  docker: boolean;\n\
  setup: boolean;\n\
}}\n\
export const foundationOperations = {{\n\
{operations}\n\
}} as const;\n"
    )
}

fn write_or_check(
    path: &Path,
    contents: &[u8],
    check: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if check {
        let existing = fs::read(path)
            .map_err(|error| format!("cannot verify generated file {}: {error}", path.display()))?;
        if existing != contents {
            return Err(format!("{} is stale; run `cargo xtask openapi`", path.display()).into());
        }
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, contents)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deployment_update_behavior_is_optional_and_defaults_to_disabled() {
        for document in [document(false), document(true)] {
            let schema = &document["components"]["schemas"]["DeploymentSpec"];
            assert_eq!(schema["required"], json!(["image"]));
            assert_eq!(
                schema["properties"]["updateBehavior"]["default"],
                "Disabled"
            );
        }
    }

    #[test]
    fn generated_documents_do_not_contain_dangling_schema_references() {
        for document in [document(false), document(true)] {
            let schemas = document["components"]["schemas"]
                .as_object()
                .expect("generated schemas are an object");
            assert_schema_references_resolve(&document, schemas);
        }
    }

    #[test]
    fn generated_path_parameters_match_their_contracts() {
        let document = document(false);
        for route in ROUTES.iter().filter(|route| route.path.contains('{')) {
            let operation = &document["paths"][route.path][route.method.as_openapi_str()];
            let parameters = operation["parameters"]
                .as_array()
                .expect("parameterized routes declare path parameters");
            let path_parameters = parameters
                .iter()
                .filter(|parameter| parameter["in"] == "path")
                .collect::<Vec<_>>();
            assert_eq!(path_parameters.len(), route.path.matches('{').count());
            for parameter in path_parameters {
                let name = parameter["name"]
                    .as_str()
                    .expect("path parameter has a name");
                let contract = route
                    .parameters
                    .iter()
                    .find(|candidate| {
                        candidate.location.as_openapi_str() == "path" && candidate.name == name
                    })
                    .expect("generated path parameter has contract metadata");
                assert_eq!(parameter["required"], true);
                assert_eq!(parameter["schema"], parameter_schema(contract.schema));
            }
        }
    }

    #[test]
    fn generated_users_operations_declare_known_problem_responses() {
        let document = document(false);
        let list = &document["paths"]["/api/v1/users"]["get"]["responses"];
        let create = &document["paths"]["/api/v1/users"]["post"]["responses"];

        assert_eq!(
            error_statuses(&json!({ "responses": list })),
            BTreeSet::from([400, 401, 403, 429, 500])
        );
        assert_eq!(
            error_statuses(&json!({ "responses": create })),
            BTreeSet::from([400, 401, 403, 409, 429, 500])
        );
        for response in [list, create] {
            for status in response
                .as_object()
                .expect("responses are an object")
                .keys()
                .filter(|status| status.parse::<u16>().is_ok_and(|status| status >= 400))
            {
                assert_eq!(
                    response[status]["content"]["application/problem+json"]["schema"]["$ref"],
                    "#/components/schemas/ProblemDetails"
                );
            }
        }
    }

    #[test]
    fn rust_api_subset_matches_the_current_frontend_contract() {
        let rust_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("xtask must be inside the Rust workspace");
        verify_frontend_contract(rust_root).unwrap();
    }

    fn assert_schema_references_resolve(value: &Value, schemas: &Map<String, Value>) {
        match value {
            Value::Array(values) => {
                for value in values {
                    assert_schema_references_resolve(value, schemas);
                }
            }
            Value::Object(values) => {
                if let Some(reference) = values.get("$ref").and_then(Value::as_str)
                    && let Some(schema) = reference.strip_prefix("#/components/schemas/")
                {
                    assert!(schemas.contains_key(schema), "missing schema: {schema}");
                }
                for value in values.values() {
                    assert_schema_references_resolve(value, schemas);
                }
            }
            _ => {}
        }
    }
}
