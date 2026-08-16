use std::fs;
use std::path::Path;

use citadel_contracts::http::{ROUTES, RouteAuthentication, RouteContract};
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
    println!(
        "{} {} full and {} public HTTP contracts",
        if check { "verified" } else { "generated" },
        ROUTES.len(),
        ROUTES.iter().filter(|route| route.public).count()
    );
    Ok(())
}

fn document(public_only: bool) -> Value {
    let mut paths = Map::new();
    for route in ROUTES.iter().filter(|route| !public_only || route.public) {
        let path = paths
            .entry(route.path.to_owned())
            .or_insert_with(|| Value::Object(Map::new()));
        path.as_object_mut()
            .expect("generated path is an object")
            .insert(route.method.to_owned(), operation(route));
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
    json!({
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
        "SetupStatusResponse": {
            "type": "object", "required": ["requiresSetup"], "additionalProperties": false,
            "properties": { "requiresSetup": { "type": "boolean" } }
        },
        "InitializeCitadelRequest": {
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
        "AccessTokenResponse": {
            "type": "object", "required": ["accessToken"], "additionalProperties": false,
            "properties": { "accessToken": string() }
        },
        "PermissionMatrixResponse": {
            "type": "array", "items": { "$ref": "#/components/schemas/PermissionMatrixEntry" }
        },
        "PermissionMatrixEntry": {
            "type": "object", "required": ["resourceType", "maximumLevel", "specificPermissions"], "additionalProperties": false,
            "properties": {
                "resourceType": { "$ref": "#/components/schemas/ResourceType" },
                "maximumLevel": { "$ref": "#/components/schemas/PermissionLevel" },
                "specificPermissions": { "type": "array", "items": { "$ref": "#/components/schemas/SpecificPermissionEntry" } }
            }
        },
        "SpecificPermissionEntry": {
            "type": "object", "required": ["permission", "minimumLevel"], "additionalProperties": false,
            "properties": {
                "permission": { "$ref": "#/components/schemas/SpecificPermission" },
                "minimumLevel": { "$ref": "#/components/schemas/PermissionLevel" }
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
        "UpdateCurrentProfileRequest": {
            "type": "object", "required": ["displayName"], "additionalProperties": false,
            "properties": { "displayName": string() }
        },
        "UserDateTimeFormat": {
            "type": "string", "enum": ["System", "TwentyFourHour", "TwelveHour"]
        },
        "UserTheme": {
            "type": "string", "enum": ["System", "Light", "Dark"]
        },
        "PatchUserPreferencesRequest": {
            "type": "object", "minProperties": 1, "additionalProperties": false,
            "properties": {
                "timeZone": { "type": "string" },
                "dateTimeFormat": { "$ref": "#/components/schemas/UserDateTimeFormat" },
                "theme": { "$ref": "#/components/schemas/UserTheme" }
            }
        },
        "ChangeCurrentPasswordRequest": {
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
        "CreateServiceAccountRequest": {
            "type": "object", "required": ["name"], "additionalProperties": false,
            "properties": {
                "name": string(), "description": nullable_string(), "isEnabled": { "type": "boolean" },
                "teamIds": uuid_array(), "roleIds": uuid_array(),
                "resourceAccesses": { "type": "array", "items": { "$ref": "#/components/schemas/ServiceAccountResourceAccess" } }
            }
        },
        "UpdateServiceAccountRequest": {
            "type": "object", "additionalProperties": false,
            "properties": { "description": nullable_string(), "isEnabled": { "type": ["boolean", "null"] } }
        },
        "RenameServiceAccountRequest": {
            "type": "object", "required": ["id", "name"], "additionalProperties": false,
            "properties": { "id": uuid(), "name": string() }
        },
        "AddServiceAccountRoleRequest": {
            "type": "object", "required": ["roleId"], "additionalProperties": false,
            "properties": { "roleId": uuid() }
        },
        "AddServiceAccountResourceAccessRequest": {
            "type": "object", "required": ["resourceType", "resourceId", "permissionLevel"], "additionalProperties": false,
            "properties": {
                "resourceType": { "$ref": "#/components/schemas/ResourceType" }, "resourceId": uuid(),
                "permissionLevel": { "$ref": "#/components/schemas/PermissionLevel" },
                "specificPermissions": { "type": "array", "items": { "$ref": "#/components/schemas/SpecificPermission" } }
            }
        },
        "ArchiveServiceAccountsRequest": {
            "type": "object", "required": ["ids"], "additionalProperties": false,
            "properties": { "ids": uuid_array() }
        },
        "ServiceAccountView": service_account_view(false),
        "ServiceAccountDetailResponse": service_account_view(true),
        "ResourceCapabilities": capabilities(false),
        "ServiceAccountCapabilities": capabilities(true),
        "ServiceAccountsResponse": {
            "type": "object", "required": ["pagedResult", "capabilities"], "additionalProperties": false,
            "properties": {
                "pagedResult": paged("ServiceAccountView"),
                "capabilities": { "$ref": "#/components/schemas/ResourceCapabilities" }
            }
        },
        "CreateServiceAccountTokenRequest": {
            "type": "object", "required": ["name"], "additionalProperties": false,
            "properties": {
                "name": string(), "expiresAtUtc": nullable_date_time(), "neverExpires": { "type": "boolean" }
            }
        },
        "ServiceAccountTokenView": service_account_token_view(false),
        "CreatedServiceAccountTokenView": service_account_token_view(true),
        "ServiceAccountTokensResponse": {
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
        }
    })
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

fn service_account_view(with_capabilities: bool) -> Value {
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
    let mut required = vec![
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
    if with_capabilities {
        properties.insert(
            "capabilities".to_owned(),
            json!({ "$ref": "#/components/schemas/ServiceAccountCapabilities" }),
        );
        required.push("capabilities");
    }
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
            json!({
                "description": "Success",
                "content": { "application/json": { "schema": { "$ref": format!("#/components/schemas/{schema}") } } }
            })
        },
    );
    let mut responses = Map::new();
    responses.insert(route.success_status.to_string(), success);
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
    let path_parameters = uuid_path_parameters(route.path);
    if !path_parameters.is_empty() {
        operation
            .as_object_mut()
            .expect("operation is an object")
            .insert("parameters".to_owned(), Value::Array(path_parameters));
    }
    operation
}

fn uuid_path_parameters(path: &str) -> Vec<Value> {
    path.split('{')
        .skip(1)
        .filter_map(|remainder| remainder.split_once('}').map(|(name, _)| name))
        .map(|name| {
            json!({
                "name": name,
                "in": "path",
                "required": true,
                "schema": { "type": "string", "format": "uuid" }
            })
        })
        .collect()
}

fn frontend_types() -> String {
    let operations = ROUTES
        .iter()
        .map(|route| {
            format!(
                "  {}: {{ method: '{}', path: '{}' }},",
                route.operation_id,
                route.method.to_ascii_uppercase(),
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
    fn generated_documents_do_not_contain_dangling_schema_references() {
        for document in [document(false), document(true)] {
            let schemas = document["components"]["schemas"]
                .as_object()
                .expect("generated schemas are an object");
            assert_schema_references_resolve(&document, schemas);
        }
    }

    #[test]
    fn generated_uuid_path_parameters_are_explicit() {
        let document = document(false);
        for route in ROUTES.iter().filter(|route| route.path.contains('{')) {
            assert!(
                route
                    .path
                    .split('{')
                    .skip(1)
                    .filter_map(|value| value.split_once('}').map(|(name, _)| name))
                    .all(|name| name.to_ascii_lowercase().ends_with("id")),
                "UUID path inference requires an ID-named parameter: {}",
                route.path
            );
            let operation = &document["paths"][route.path][route.method];
            let parameters = operation["parameters"]
                .as_array()
                .expect("parameterized routes declare path parameters");
            assert_eq!(parameters.len(), route.path.matches('{').count());
            assert!(parameters.iter().all(|parameter| {
                parameter["in"] == "path"
                    && parameter["required"] == true
                    && parameter["schema"]["format"] == "uuid"
            }));
        }
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
