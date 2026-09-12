use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_yaml_ng::{Mapping, Value};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::{StackBuildImageBinding, StackError, validation};

const MAX_COMPOSE_FILES: usize = 16;
const MAX_COMPOSE_BYTES: usize = 4 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SwarmStackCompatibilityIssue {
    pub severity: SwarmStackCompatibilitySeverity,
    pub code: String,
    pub message: String,
    pub field_path: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub enum SwarmStackCompatibilitySeverity {
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SwarmStackCompatibilityReport {
    pub is_compatible: bool,
    pub issues: Vec<SwarmStackCompatibilityIssue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComposeService {
    pub name: String,
    pub image: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComposeModel {
    pub services: Vec<ComposeService>,
    pub named_volumes: BTreeSet<String>,
    pub volume_names: BTreeMap<String, String>,
    pub external_volumes: BTreeSet<String>,
    pub service_volumes: BTreeMap<String, BTreeSet<String>>,
    pub external_secrets: BTreeMap<String, String>,
    pub external_configs: BTreeMap<String, String>,
    pub variables: BTreeSet<String>,
    pub optional_variables: BTreeSet<String>,
}

pub fn analyze_swarm_compatibility(
    files: &[String],
    bindings: &[StackBuildImageBinding],
) -> Result<SwarmStackCompatibilityReport, StackError> {
    let mut issues = Vec::new();
    if files.is_empty() {
        issues.push(error(
            "compose.empty",
            "services",
            "At least one Compose file is required.",
        ));
        return Ok(compatibility_report(issues));
    }
    if files.len() > MAX_COMPOSE_FILES {
        issues.push(error(
            "compose.too_many_files",
            "files",
            "Swarm Stack preflight accepts at most 16 Compose files.",
        ));
        return Ok(compatibility_report(issues));
    }
    let total_bytes = files.iter().try_fold(0usize, |total, file| {
        total.checked_add(file.len()).ok_or(())
    });
    if total_bytes.is_err() || total_bytes.is_ok_and(|bytes| bytes > MAX_COMPOSE_BYTES) {
        issues.push(error(
            "compose.too_large",
            "files",
            "The combined Compose input exceeds the 4 MiB preflight limit.",
        ));
        return Ok(compatibility_report(issues));
    }
    let mut service_names = BTreeSet::new();
    let mut has_image = BTreeSet::new();
    let build_services = bindings
        .iter()
        .map(|binding| binding.service_name.to_ascii_lowercase())
        .collect::<BTreeSet<_>>();

    for (file_index, content) in files.iter().enumerate() {
        let prefix = if files.len() == 1 {
            String::new()
        } else {
            format!("files[{file_index}].")
        };
        let document: Value = match serde_yaml_ng::from_str(content) {
            Ok(Value::Mapping(document)) => Value::Mapping(document),
            Ok(_) => {
                issues.push(error(
                    "compose.invalid_root",
                    prefix.trim_end_matches('.'),
                    "Compose must contain a YAML mapping at its root.",
                ));
                continue;
            }
            Err(error_value) => {
                issues.push(error(
                    "compose.invalid_yaml",
                    prefix.trim_end_matches('.'),
                    &format!("Compose YAML is invalid: {error_value}"),
                ));
                continue;
            }
        };
        let root = document.as_mapping().expect("matched a mapping above");
        validate_allowed_keys(
            root,
            &[
                "version", "services", "networks", "volumes", "secrets", "configs",
            ],
            prefix.trim_end_matches('.'),
            true,
            &mut issues,
        );
        validate_top_level_resources(&document, "networks", &mut issues);
        validate_top_level_resources(&document, "volumes", &mut issues);
        validate_top_level_resources(&document, "secrets", &mut issues);
        validate_top_level_resources(&document, "configs", &mut issues);
        let Some(raw_services) = root.get(Value::String("services".to_owned())) else {
            continue;
        };
        let Some(services) = raw_services.as_mapping() else {
            issues.push(error(
                "compose.invalid_structure",
                &format!("{prefix}services"),
                "'services' must be a mapping.",
            ));
            continue;
        };
        for (name, service) in services {
            let Some(name) = name.as_str() else {
                issues.push(error(
                    "service.invalid",
                    &format!("{prefix}services"),
                    "Service names must be strings.",
                ));
                continue;
            };
            service_names.insert(name.to_owned());
            let path = format!("{prefix}services.{name}");
            let Some(service) = service.as_mapping() else {
                issues.push(error(
                    "service.invalid",
                    &path,
                    "Service definitions must be mappings.",
                ));
                continue;
            };
            if scalar_at(service, "image").is_some() {
                has_image.insert(name.to_ascii_lowercase());
            }
            validate_allowed_keys(
                service,
                &[
                    "image",
                    "build",
                    "command",
                    "entrypoint",
                    "working_dir",
                    "user",
                    "environment",
                    "env_file",
                    "labels",
                    "healthcheck",
                    "hostname",
                    "stop_grace_period",
                    "logging",
                    "deploy",
                    "endpoint_mode",
                    "networks",
                    "volumes",
                    "ports",
                    "secrets",
                    "configs",
                    "tmpfs",
                ],
                &path,
                false,
                &mut issues,
            );
            validate_reserved_labels(service, &format!("{path}.labels"), &mut issues);
            if service.contains_key(Value::String("build".to_owned()))
                && !build_services.contains(&name.to_ascii_lowercase())
            {
                issues.push(error(
                    "service.build_unsupported",
                    &format!("{path}.build"),
                    "Swarm Stack builds require a matching Citadel build binding.",
                ));
            }
            validate_deploy(service, &path, &mut issues);
            validate_mounts(service, &path, &mut issues);
            validate_ports(service, &path, &mut issues);
        }
    }
    if service_names.is_empty() {
        issues.push(error(
            "services.empty",
            "services",
            "A Stack must define at least one Service.",
        ));
    }
    for service in &service_names {
        if !has_image.contains(&service.to_ascii_lowercase())
            && !build_services.contains(&service.to_ascii_lowercase())
        {
            issues.push(error(
                "service.image_required",
                &format!("services.{service}.image"),
                &format!("Service '{service}' requires an image or Citadel build binding."),
            ));
        }
    }
    Ok(compatibility_report(issues))
}

pub fn parse_compose(files: &[String]) -> Result<ComposeModel, StackError> {
    validate_input_limits(files)?;
    let mut model = ComposeModel {
        services: Vec::new(),
        named_volumes: BTreeSet::new(),
        volume_names: BTreeMap::new(),
        external_volumes: BTreeSet::new(),
        service_volumes: BTreeMap::new(),
        external_secrets: BTreeMap::new(),
        external_configs: BTreeMap::new(),
        variables: BTreeSet::new(),
        optional_variables: BTreeSet::new(),
    };
    let mut merged_services = BTreeMap::<String, Option<String>>::new();
    for content in files {
        collect_variables(content, &mut model.variables, &mut model.optional_variables);
        let document = parse_document(content)?;
        if let Some(services) = mapping_at(&document, "services") {
            for (name, definition) in services {
                let Some(name) = name.as_str() else { continue };
                let image = definition
                    .as_mapping()
                    .and_then(|map| scalar_at(map, "image"))
                    .map(str::to_owned);
                if let Some(definition) = definition.as_mapping() {
                    collect_service_volumes(
                        definition,
                        model.service_volumes.entry(name.to_owned()).or_default(),
                    );
                }
                merged_services
                    .entry(name.to_owned())
                    .and_modify(|current| {
                        if image.is_some() {
                            *current = image.clone();
                        }
                    })
                    .or_insert(image);
            }
        }
        collect_named_resources(&document, "volumes", &mut model.named_volumes);
        collect_volume_metadata(
            &document,
            &mut model.volume_names,
            &mut model.external_volumes,
        );
        collect_external_resources(&document, "secrets", &mut model.external_secrets);
        collect_external_resources(&document, "configs", &mut model.external_configs);
    }
    model.services = merged_services
        .into_iter()
        .map(|(name, image)| ComposeService { name, image })
        .collect();
    if model.services.is_empty() {
        return Err(validation("A Stack must define at least one Service."));
    }
    Ok(model)
}

fn collect_service_volumes(service: &Mapping, target: &mut BTreeSet<String>) {
    let Some(volumes) = service
        .get(Value::String("volumes".to_owned()))
        .and_then(Value::as_sequence)
    else {
        return;
    };
    for volume in volumes {
        let source = match volume {
            Value::String(value) => value.split(':').next(),
            Value::Mapping(value)
                if scalar_at(value, "type")
                    .is_none_or(|kind| kind.eq_ignore_ascii_case("volume")) =>
            {
                scalar_at(value, "source")
            }
            _ => None,
        };
        if let Some(source) = source.filter(|value| is_named_volume_reference(value)) {
            target.insert(source.to_owned());
        }
    }
}

fn is_named_volume_reference(value: &str) -> bool {
    !value.is_empty()
        && !value.starts_with('.')
        && !value.starts_with('/')
        && !value.starts_with('~')
        && !value.contains('\\')
}

fn collect_volume_metadata(
    document: &Value,
    names: &mut BTreeMap<String, String>,
    external: &mut BTreeSet<String>,
) {
    let Some(volumes) = mapping_at(document, "volumes") else {
        return;
    };
    for (key, definition) in volumes {
        let Some(key) = key.as_str() else { continue };
        let Some(definition) = definition.as_mapping() else {
            continue;
        };
        let external_value = definition.get(Value::String("external".to_owned()));
        let is_external = matches!(
            external_value,
            Some(Value::Bool(true)) | Some(Value::Mapping(_))
        );
        let name = scalar_at(definition, "name")
            .or_else(|| {
                external_value
                    .and_then(Value::as_mapping)
                    .and_then(|value| scalar_at(value, "name"))
            })
            .unwrap_or(key);
        names.insert(key.to_owned(), name.to_owned());
        if is_external {
            external.insert(key.to_owned());
        }
    }
}

pub fn inject_ownership_labels(
    compose: &str,
    stack_id: Uuid,
    release_id: Uuid,
    include_swarm_service_labels: bool,
) -> Result<String, StackError> {
    let mut document = parse_document(compose)?;
    let services = mapping_at_mut(&mut document, "services")
        .ok_or_else(|| validation("A Stack must define at least one Service."))?;
    let service_count = services.len();
    for (name, service) in services {
        let service_name = name
            .as_str()
            .ok_or_else(|| validation("Service names must be strings."))?
            .to_owned();
        let service = service
            .as_mapping_mut()
            .ok_or_else(|| validation("Service definitions must be mappings."))?;
        reject_reserved_labels(service, &service_name)?;
        let hash = service_hash(service)?;
        set_ownership_labels(
            labels_entry(service, &service_name)?,
            stack_id,
            release_id,
            &hash,
            service_count,
        );
        if include_swarm_service_labels {
            let deploy = mapping_entry(service, "deploy")?;
            set_ownership_labels(
                labels_entry(deploy, &service_name)?,
                stack_id,
                release_id,
                &hash,
                service_count,
            );
        }
    }
    serde_yaml_ng::to_string(&document).map_err(|error| StackError::Validation(error.to_string()))
}

/// Creates the final Compose override used for Git-backed Stacks. Keeping the
/// generated labels in a separate file preserves the immutable repository
/// snapshot and lets Docker apply normal multi-file Compose merge semantics.
pub fn create_ownership_labels_override(
    compose_files: &[String],
    stack_id: Uuid,
    release_id: Uuid,
    include_swarm_service_labels: bool,
) -> Result<String, StackError> {
    validate_input_limits(compose_files)?;
    let mut compose_version = None;
    let mut definitions = BTreeMap::<String, Vec<String>>::new();
    for content in compose_files {
        let document = parse_document(content)?;
        if include_swarm_service_labels && compose_version.is_none() {
            compose_version = document.get("version").cloned();
        }
        let Some(services) = mapping_at(&document, "services") else {
            continue;
        };
        for (name, service) in services {
            let name = name
                .as_str()
                .filter(|value| !value.trim().is_empty())
                .ok_or_else(|| validation("Service names must be non-empty strings."))?;
            let service = service
                .as_mapping()
                .ok_or_else(|| validation("Service definitions must be mappings."))?;
            reject_reserved_labels(service, name)?;
            definitions.entry(name.to_owned()).or_default().push(
                serde_yaml_ng::to_string(&Value::Mapping(service.clone())).map_err(|error| {
                    validation(&format!("Could not hash Compose Service: {error}"))
                })?,
            );
        }
    }
    if definitions.is_empty() {
        return Err(validation("A Stack must define at least one Service."));
    }
    let service_count = definitions.len();
    let mut services = Mapping::new();
    for (name, service_definitions) in definitions {
        let mut hash = Sha256::new();
        for definition in service_definitions {
            hash.update((definition.len() as u64).to_be_bytes());
            hash.update(definition.as_bytes());
        }
        let service_hash = hash
            .finalize()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        let mut service = Mapping::new();
        let mut labels = Mapping::new();
        set_ownership_labels(
            &mut labels,
            stack_id,
            release_id,
            &service_hash,
            service_count,
        );
        service.insert(Value::String("labels".to_owned()), Value::Mapping(labels));
        if include_swarm_service_labels {
            let mut deploy = Mapping::new();
            let mut labels = Mapping::new();
            set_ownership_labels(
                &mut labels,
                stack_id,
                release_id,
                &service_hash,
                service_count,
            );
            deploy.insert(Value::String("labels".to_owned()), Value::Mapping(labels));
            service.insert(Value::String("deploy".to_owned()), Value::Mapping(deploy));
        }
        services.insert(Value::String(name), Value::Mapping(service));
    }
    let mut root = Mapping::new();
    // Docker stack deploy requires every input file, including generated
    // overrides, to use the same Compose version as the user's source.
    if let Some(version) = compose_version {
        root.insert(Value::String("version".into()), version);
    }
    root.insert(
        Value::String("services".to_owned()),
        Value::Mapping(services),
    );
    serde_yaml_ng::to_string(&Value::Mapping(root))
        .map_err(|error| validation(&format!("Could not create labels override: {error}")))
}

#[must_use]
pub fn compose_digest(files: &[String]) -> String {
    let mut hash = Sha256::new();
    for file in files {
        hash.update((file.len() as u64).to_be_bytes());
        hash.update(file.as_bytes());
    }
    let digest = hash.finalize();
    let encoded = digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("sha256:{encoded}")
}

fn validate_input_limits(files: &[String]) -> Result<(), StackError> {
    if files.is_empty() || files.len() > MAX_COMPOSE_FILES {
        return Err(validation("Between 1 and 16 Compose files are required."));
    }
    let bytes = files
        .iter()
        .try_fold(0usize, |total, file| total.checked_add(file.len()))
        .ok_or_else(|| validation("Compose input exceeds the supported size."))?;
    if bytes > MAX_COMPOSE_BYTES {
        return Err(validation("Compose input must not exceed 4 MiB."));
    }
    Ok(())
}

fn parse_document(content: &str) -> Result<Value, StackError> {
    let value: Value = serde_yaml_ng::from_str(content)
        .map_err(|error| validation(&format!("Invalid Compose YAML: {error}")))?;
    if !value.is_mapping() {
        return Err(validation("The Compose document must be a mapping."));
    }
    Ok(value)
}

fn mapping_at<'a>(value: &'a Value, name: &str) -> Option<&'a Mapping> {
    value
        .as_mapping()?
        .get(Value::String(name.to_owned()))?
        .as_mapping()
}

fn mapping_at_mut<'a>(value: &'a mut Value, name: &str) -> Option<&'a mut Mapping> {
    value
        .as_mapping_mut()?
        .get_mut(Value::String(name.to_owned()))?
        .as_mapping_mut()
}

fn mapping_entry<'a>(mapping: &'a mut Mapping, key: &str) -> Result<&'a mut Mapping, StackError> {
    let key = Value::String(key.to_owned());
    if !mapping.contains_key(&key) {
        mapping.insert(key.clone(), Value::Mapping(Mapping::new()));
    }
    mapping
        .get_mut(&key)
        .and_then(Value::as_mapping_mut)
        .ok_or_else(|| validation(&format!("'{key:?}' must be a mapping.")))
}

fn labels_entry<'a>(
    mapping: &'a mut Mapping,
    service_name: &str,
) -> Result<&'a mut Mapping, StackError> {
    let key = Value::String("labels".to_owned());
    if !mapping.contains_key(&key) {
        mapping.insert(key.clone(), Value::Mapping(Mapping::new()));
    }
    if matches!(mapping.get(&key), Some(Value::Sequence(_))) {
        let sequence = mapping
            .remove(&key)
            .and_then(|value| value.as_sequence().cloned())
            .unwrap_or_default();
        let mut labels = Mapping::new();
        for value in sequence {
            let value = value.as_str().ok_or_else(|| {
                validation(&format!(
                    "Labels for Service '{service_name}' must contain strings."
                ))
            })?;
            let (name, label_value) = value.split_once('=').unwrap_or((value, ""));
            labels.insert(
                Value::String(name.to_owned()),
                Value::String(label_value.to_owned()),
            );
        }
        mapping.insert(key.clone(), Value::Mapping(labels));
    }
    mapping
        .get_mut(&key)
        .and_then(Value::as_mapping_mut)
        .ok_or_else(|| {
            validation(&format!(
                "Labels for Service '{service_name}' must be a mapping or sequence."
            ))
        })
}

fn set_ownership_labels(
    labels: &mut Mapping,
    stack_id: Uuid,
    release_id: Uuid,
    service_hash: &str,
    service_count: usize,
) {
    for (key, value) in [
        ("com.citadel.managed", "true".to_owned()),
        ("com.citadel.stack-id", stack_id.to_string()),
        ("com.citadel.release-id", release_id.to_string()),
        ("com.citadel.service-hash", service_hash.to_owned()),
        ("com.citadel.stack-service-count", service_count.to_string()),
    ] {
        labels.insert(Value::String(key.to_owned()), Value::String(value));
    }
}

fn service_hash(service: &Mapping) -> Result<String, StackError> {
    let serialized = serde_yaml_ng::to_string(&Value::Mapping(service.clone()))
        .map_err(|error| validation(&format!("Could not hash Compose Service: {error}")))?;
    let digest = Sha256::digest(serialized.as_bytes());
    Ok(digest.iter().map(|byte| format!("{byte:02x}")).collect())
}

fn scalar_at<'a>(mapping: &'a Mapping, key: &str) -> Option<&'a str> {
    mapping
        .get(Value::String(key.to_owned()))
        .and_then(Value::as_str)
}

fn validate_top_level_resources(
    document: &Value,
    section: &str,
    issues: &mut Vec<SwarmStackCompatibilityIssue>,
) {
    let Some(raw_resources) = document
        .as_mapping()
        .and_then(|mapping| mapping.get(Value::String(section.to_owned())))
    else {
        return;
    };
    let Some(resources) = raw_resources.as_mapping() else {
        issues.push(error(
            "compose.invalid_structure",
            section,
            &format!("'{section}' must be a mapping."),
        ));
        return;
    };
    let allowed = match section {
        "networks" => &[
            "driver",
            "driver_opts",
            "attachable",
            "external",
            "name",
            "labels",
            "internal",
            "enable_ipv4",
            "enable_ipv6",
        ][..],
        "volumes" => &["driver", "driver_opts", "external", "name", "labels"][..],
        "secrets" | "configs" => &[
            "file",
            "external",
            "name",
            "labels",
            "driver",
            "template_driver",
        ][..],
        _ => &[][..],
    };
    for (name, value) in resources {
        let path = format!("{section}.{}", name.as_str().unwrap_or("?"));
        if !value.is_mapping() && !value.is_null() {
            issues.push(error(
                "compose.invalid_structure",
                &path,
                &format!("'{path}' must be a mapping."),
            ));
        }
        if let Some(definition) = value.as_mapping() {
            validate_allowed_keys(definition, allowed, &path, true, issues);
            validate_reserved_labels(definition, &format!("{path}.labels"), issues);
        }
        if section == "volumes"
            && value
                .as_mapping()
                .and_then(|mapping| scalar_at(mapping, "driver"))
                .is_some_and(|driver| driver.eq_ignore_ascii_case("local"))
        {
            issues.push(warning(
                "volume.local_portability",
                &path,
                "A node-local Volume is not portable between Swarm nodes.",
            ));
        }
    }
}

fn compatibility_report(
    mut issues: Vec<SwarmStackCompatibilityIssue>,
) -> SwarmStackCompatibilityReport {
    issues.sort_by(|left, right| {
        left.field_path
            .cmp(&right.field_path)
            .then(left.code.cmp(&right.code))
    });
    SwarmStackCompatibilityReport {
        is_compatible: !issues
            .iter()
            .any(|issue| issue.severity == SwarmStackCompatibilitySeverity::Error),
        issues,
    }
}

fn validate_allowed_keys(
    mapping: &Mapping,
    allowed: &[&str],
    parent_path: &str,
    allow_extensions: bool,
    issues: &mut Vec<SwarmStackCompatibilityIssue>,
) {
    for key in mapping.keys().filter_map(Value::as_str) {
        if allowed
            .iter()
            .any(|allowed| key.eq_ignore_ascii_case(allowed))
            || (allow_extensions
                && key.starts_with("x-")
                && !key.to_ascii_lowercase().starts_with("x-citadel."))
        {
            continue;
        }
        let path = if parent_path.is_empty() {
            key.to_owned()
        } else {
            format!("{parent_path}.{key}")
        };
        issues.push(error(
            "compose.unsupported_key",
            &path,
            &format!("'{path}' is not supported by Swarm Stacks."),
        ));
    }
}

fn validate_reserved_labels(
    parent: &Mapping,
    path: &str,
    issues: &mut Vec<SwarmStackCompatibilityIssue>,
) {
    let Some(labels) = parent.get(Value::String("labels".to_owned())) else {
        return;
    };
    let names = match labels {
        Value::Mapping(mapping) => mapping
            .keys()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect::<Vec<_>>(),
        Value::Sequence(sequence) => sequence
            .iter()
            .filter_map(Value::as_str)
            .map(|label| {
                label
                    .split_once('=')
                    .map_or(label, |value| value.0)
                    .to_owned()
            })
            .collect(),
        _ => Vec::new(),
    };
    for name in names.into_iter().filter(|name| is_reserved(name)) {
        issues.push(error(
            "labels.reserved",
            path,
            &format!("Label '{name}' is reserved for Citadel ownership."),
        ));
    }
}

fn validate_deploy(
    service: &Mapping,
    service_path: &str,
    issues: &mut Vec<SwarmStackCompatibilityIssue>,
) {
    let Some(deploy) = service.get(Value::String("deploy".to_owned())) else {
        return;
    };
    let path = format!("{service_path}.deploy");
    let Some(deploy) = deploy.as_mapping() else {
        issues.push(error(
            "compose.invalid_structure",
            &path,
            "'deploy' must be a mapping.",
        ));
        return;
    };
    validate_allowed_keys(
        deploy,
        &[
            "mode",
            "replicas",
            "placement",
            "resources",
            "restart_policy",
            "update_config",
            "rollback_config",
            "labels",
        ],
        &path,
        false,
        issues,
    );
    validate_reserved_labels(deploy, &format!("{path}.labels"), issues);
    validate_enum(deploy, "mode", &["replicated", "global"], &path, issues);
    for (key, allowed) in [
        (
            "placement",
            &["constraints", "preferences", "max_replicas_per_node"][..],
        ),
        (
            "restart_policy",
            &["condition", "delay", "max_attempts", "window"][..],
        ),
        (
            "update_config",
            &[
                "parallelism",
                "delay",
                "failure_action",
                "monitor",
                "max_failure_ratio",
                "order",
            ][..],
        ),
        (
            "rollback_config",
            &[
                "parallelism",
                "delay",
                "failure_action",
                "monitor",
                "max_failure_ratio",
                "order",
            ][..],
        ),
    ] {
        validate_nested_mapping(deploy, key, allowed, &path, issues);
    }
}

fn validate_nested_mapping(
    parent: &Mapping,
    key: &str,
    allowed: &[&str],
    parent_path: &str,
    issues: &mut Vec<SwarmStackCompatibilityIssue>,
) {
    let Some(value) = parent.get(Value::String(key.to_owned())) else {
        return;
    };
    let path = format!("{parent_path}.{key}");
    let Some(mapping) = value.as_mapping() else {
        issues.push(error(
            "compose.invalid_structure",
            &path,
            &format!("'{key}' must be a mapping."),
        ));
        return;
    };
    validate_allowed_keys(mapping, allowed, &path, false, issues);
}

fn validate_enum(
    mapping: &Mapping,
    key: &str,
    allowed: &[&str],
    parent_path: &str,
    issues: &mut Vec<SwarmStackCompatibilityIssue>,
) {
    let Some(value) = scalar_at(mapping, key) else {
        return;
    };
    if !allowed
        .iter()
        .any(|allowed| value.eq_ignore_ascii_case(allowed))
    {
        let path = format!("{parent_path}.{key}");
        issues.push(error(
            "compose.unsupported_value",
            &path,
            &format!("'{path}' has unsupported value '{value}'."),
        ));
    }
}

fn validate_mounts(
    service: &Mapping,
    service_path: &str,
    issues: &mut Vec<SwarmStackCompatibilityIssue>,
) {
    let Some(volumes) = service.get(Value::String("volumes".to_owned())) else {
        return;
    };
    let root_path = format!("{service_path}.volumes");
    let Some(volumes) = volumes.as_sequence() else {
        issues.push(error(
            "compose.invalid_structure",
            &root_path,
            "'volumes' must be a list.",
        ));
        return;
    };
    for (index, volume) in volumes.iter().enumerate() {
        let path = format!("{root_path}[{index}]");
        let is_bind = match volume {
            Value::String(value) => {
                let source = value.split(':').next().unwrap_or_default();
                source.starts_with('.') || source.starts_with('/') || source.contains('\\')
            }
            Value::Mapping(mapping) => {
                scalar_at(mapping, "type").is_some_and(|kind| kind.eq_ignore_ascii_case("bind"))
            }
            _ => {
                issues.push(error(
                    "compose.invalid_structure",
                    &path,
                    "A volume reference must be a string or mapping.",
                ));
                false
            }
        };
        if is_bind {
            issues.push(warning(
                "mount.bind_portability",
                &path,
                "Bind mounts require the same host path on every eligible Swarm node.",
            ));
        }
    }
}

fn validate_ports(
    service: &Mapping,
    service_path: &str,
    issues: &mut Vec<SwarmStackCompatibilityIssue>,
) {
    let Some(ports) = service.get(Value::String("ports".to_owned())) else {
        return;
    };
    let root_path = format!("{service_path}.ports");
    let Some(ports) = ports.as_sequence() else {
        issues.push(error(
            "compose.invalid_structure",
            &root_path,
            "'ports' must be a list.",
        ));
        return;
    };
    for (index, port) in ports.iter().enumerate() {
        let Value::Mapping(port) = port else { continue };
        let path = format!("{root_path}[{index}]");
        validate_allowed_keys(
            port,
            &[
                "name",
                "target",
                "published",
                "protocol",
                "app_protocol",
                "mode",
            ],
            &path,
            false,
            issues,
        );
        validate_enum(port, "mode", &["ingress", "host"], &path, issues);
        validate_enum(port, "protocol", &["tcp", "udp", "sctp"], &path, issues);
        if scalar_at(port, "mode").is_some_and(|mode| mode.eq_ignore_ascii_case("host"))
            && port.contains_key(Value::String("published".to_owned()))
        {
            issues.push(warning(
                "port.host_mode_portability",
                &path,
                "A fixed Host-mode published port can schedule only where that port is available.",
            ));
        }
    }
}

fn collect_named_resources(document: &Value, section: &str, target: &mut BTreeSet<String>) {
    if let Some(resources) = mapping_at(document, section) {
        target.extend(
            resources
                .keys()
                .filter_map(Value::as_str)
                .map(str::to_owned),
        );
    }
}

fn collect_external_resources(
    document: &Value,
    section: &str,
    target: &mut BTreeMap<String, String>,
) {
    let Some(resources) = mapping_at(document, section) else {
        return;
    };
    for (key, value) in resources {
        let Some(key) = key.as_str() else { continue };
        let Some(definition) = value.as_mapping() else {
            continue;
        };
        let external = definition.get(Value::String("external".to_owned()));
        let is_external = matches!(external, Some(Value::Bool(true)) | Some(Value::Mapping(_)));
        if !is_external {
            continue;
        }
        let explicit_name = scalar_at(definition, "name")
            .or_else(|| {
                external
                    .and_then(Value::as_mapping)
                    .and_then(|map| scalar_at(map, "name"))
            })
            .unwrap_or(key);
        target.insert(key.to_owned(), explicit_name.to_owned());
    }
}

fn collect_variables(
    content: &str,
    target: &mut BTreeSet<String>,
    optional: &mut BTreeSet<String>,
) {
    let bytes = content.as_bytes();
    let mut index = 0;
    while index + 2 < bytes.len() {
        if bytes[index] != b'$'
            || bytes[index + 1] != b'{'
            || index
                .checked_sub(1)
                .is_some_and(|prior| bytes[prior] == b'$')
        {
            index += 1;
            continue;
        }
        let start = index + 2;
        let Some(relative_end) = content[start..].find('}') else {
            break;
        };
        let expression = &content[start..start + relative_end];
        let name = expression
            .split([':', '-', '?', '+'])
            .next()
            .unwrap_or_default();
        if !name.is_empty()
            && name
                .chars()
                .next()
                .is_some_and(|ch| ch == '_' || ch.is_ascii_alphabetic())
            && name
                .chars()
                .all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
        {
            target.insert(name.to_owned());
            let suffix = &expression[name.len()..];
            if suffix.starts_with('-')
                || suffix.starts_with(":-")
                || suffix.starts_with('+')
                || suffix.starts_with(":+")
            {
                optional.insert(name.to_owned());
            }
        }
        index = start + relative_end + 1;
    }
}

fn reject_reserved_labels(service: &Mapping, service_name: &str) -> Result<(), StackError> {
    for section in [
        Some(service),
        service
            .get(Value::String("deploy".to_owned()))
            .and_then(Value::as_mapping),
    ]
    .into_iter()
    .flatten()
    {
        let Some(labels) = section.get(Value::String("labels".to_owned())) else {
            continue;
        };
        let reserved = match labels {
            Value::Mapping(values) => values.keys().filter_map(Value::as_str).any(is_reserved),
            Value::Sequence(values) => values
                .iter()
                .filter_map(Value::as_str)
                .filter_map(|value| value.split_once('=').map(|item| item.0))
                .any(is_reserved),
            _ => {
                return Err(validation(&format!(
                    "Labels for Service '{service_name}' must be a mapping."
                )));
            }
        };
        if reserved {
            return Err(validation(&format!(
                "Service '{service_name}' uses the reserved com.citadel label namespace."
            )));
        }
    }
    Ok(())
}

fn is_reserved(value: &str) -> bool {
    let value = value.to_ascii_lowercase();
    value.starts_with("com.citadel.") || value.starts_with("x-citadel")
}

fn error(code: &str, path: &str, message: &str) -> SwarmStackCompatibilityIssue {
    SwarmStackCompatibilityIssue {
        severity: SwarmStackCompatibilitySeverity::Error,
        code: code.to_owned(),
        message: message.to_owned(),
        field_path: Some(path.to_owned()),
    }
}

fn warning(code: &str, path: &str, message: &str) -> SwarmStackCompatibilityIssue {
    SwarmStackCompatibilityIssue {
        severity: SwarmStackCompatibilitySeverity::Warning,
        code: code.to_owned(),
        message: message.to_owned(),
        field_path: Some(path.to_owned()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supported_swarm_compose_is_compatible_and_empty_resources_are_mappings() {
        let report = analyze_swarm_compatibility(
            &["services:\n  web:\n    image: nginx\nvolumes:\n  data:\n".to_owned()],
            &[],
        )
        .unwrap();
        assert!(report.is_compatible);
    }

    #[test]
    fn unsupported_fields_fail_closed_and_bind_mounts_warn() {
        let report = analyze_swarm_compatibility(&["services:\n  web:\n    image: nginx\n    container_name: web\n    volumes:\n      - ./data:/data\n".to_owned()], &[]).unwrap();
        assert!(!report.is_compatible);
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.code == "compose.unsupported_key")
        );
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.code == "mount.bind_portability")
        );
    }

    #[test]
    fn build_requires_matching_binding() {
        let report =
            analyze_swarm_compatibility(&["services:\n  api:\n    build: .\n".to_owned()], &[])
                .unwrap();
        assert!(!report.is_compatible);
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.code == "service.build_unsupported")
        );
    }

    #[test]
    fn multiple_files_can_supply_the_image() {
        let report = analyze_swarm_compatibility(
            &[
                "services:\n  api:\n    deploy:\n      replicas: 2\n".to_owned(),
                "services:\n  api:\n    image: example/api:1\n".to_owned(),
            ],
            &[],
        )
        .unwrap();
        assert!(report.is_compatible);
    }

    #[test]
    fn swarm_ownership_override_preserves_the_declared_compose_version() {
        for version in ["3.8", "3.9"] {
            let source = format!("version: '{version}'\nservices:\n  web:\n    image: alpine\n");
            let generated =
                create_ownership_labels_override(&[source], Uuid::now_v7(), Uuid::now_v7(), true)
                    .unwrap();
            let document: Value = serde_yaml_ng::from_str(&generated).unwrap();
            assert_eq!(document["version"].as_str(), Some(version));
        }
    }

    #[test]
    fn ownership_override_covers_services_across_all_compose_files() {
        let stack_id = Uuid::now_v7();
        let release_id = Uuid::now_v7();
        let override_file = create_ownership_labels_override(
            &[
                "services:\n  api:\n    image: example/api:1\n".to_owned(),
                "services:\n  api:\n    deploy:\n      replicas: 2\n  worker:\n    image: example/worker:1\n"
                    .to_owned(),
            ],
            stack_id,
            release_id,
            true,
        )
        .unwrap();
        let document: Value = serde_yaml_ng::from_str(&override_file).unwrap();
        let services = mapping_at(&document, "services").unwrap();
        assert_eq!(services.len(), 2);
        for service in services.values().filter_map(Value::as_mapping) {
            let labels = service
                .get(Value::String("labels".to_owned()))
                .and_then(Value::as_mapping)
                .unwrap();
            assert_eq!(
                labels.get(Value::String("com.citadel.stack-id".to_owned())),
                Some(&Value::String(stack_id.to_string()))
            );
            assert!(
                service
                    .get(Value::String("deploy".to_owned()))
                    .and_then(Value::as_mapping)
                    .and_then(|deploy| deploy.get(Value::String("labels".to_owned())))
                    .is_some()
            );
        }
    }

    #[test]
    fn ownership_override_rejects_reserved_labels_in_any_compose_file() {
        let error = create_ownership_labels_override(
            &[
                "services:\n  api:\n    image: nginx\n".to_owned(),
                "services:\n  api:\n    labels:\n      com.citadel.managed: fake\n".to_owned(),
            ],
            Uuid::now_v7(),
            Uuid::now_v7(),
            false,
        )
        .unwrap_err();
        assert!(error.to_string().contains("reserved com.citadel"));
    }

    #[test]
    fn parser_finds_variables_named_volumes_and_external_identity() {
        let model = parse_compose(&["services:\n  api:\n    image: nginx:${TAG:-latest}\n    environment:\n      TOKEN: ${TOKEN:?required}\n      ESCAPED: $${NOT_A_BINDING}\nvolumes:\n  data:\nsecrets:\n  token:\n    name: shared-token\n    external: true\n".to_owned()]).unwrap();
        assert_eq!(
            model.variables,
            BTreeSet::from(["TAG".to_owned(), "TOKEN".to_owned()])
        );
        assert_eq!(model.optional_variables, BTreeSet::from(["TAG".to_owned()]));
        assert!(model.named_volumes.contains("data"));
        assert_eq!(model.external_secrets["token"], "shared-token");
    }

    #[test]
    fn parser_retains_service_volume_references_and_physical_names() {
        let model = parse_compose(&["services:\n  api:\n    image: nginx\n    volumes:\n      - data:/data\n      - type: volume\n        source: shared\n        target: /shared\n      - ./host:/host\nvolumes:\n  data: {}\n  shared:\n    external: true\n    name: global-shared\n".to_owned()]).unwrap();
        assert_eq!(
            model.service_volumes["api"],
            BTreeSet::from(["data".to_owned(), "shared".to_owned()])
        );
        assert_eq!(model.volume_names["shared"], "global-shared");
        assert!(model.external_volumes.contains("shared"));
    }

    #[test]
    fn ownership_injection_rejects_reserved_user_labels() {
        let result = inject_ownership_labels(
            "services:\n  api:\n    image: nginx\n    labels:\n      com.citadel.managed: nope\n",
            Uuid::now_v7(),
            Uuid::now_v7(),
            false,
        );
        assert!(matches!(result, Err(StackError::Validation(_))));
    }

    #[test]
    fn ownership_injection_adds_swarm_service_and_task_labels() {
        let id = Uuid::now_v7();
        let release_id = Uuid::now_v7();
        let output = inject_ownership_labels(
            "services:\n  api:\n    image: nginx\n",
            id,
            release_id,
            true,
        )
        .unwrap();
        assert!(output.contains("com.citadel.stack-id"));
        assert!(output.contains(&id.to_string()));
        assert!(output.contains("com.citadel.release-id"));
        assert!(output.contains(&release_id.to_string()));
        assert!(output.contains("com.citadel.service-hash"));
        assert!(output.contains("com.citadel.stack-service-count"));
    }

    #[test]
    fn input_limits_are_bounded() {
        assert_eq!(
            analyze_swarm_compatibility(&[], &[]).unwrap().issues[0].code,
            "compose.empty"
        );
        assert_eq!(
            analyze_swarm_compatibility(&vec!["x".to_owned(); 17], &[])
                .unwrap()
                .issues[0]
                .code,
            "compose.too_many_files"
        );
        assert_eq!(
            analyze_swarm_compatibility(&["x".repeat(MAX_COMPOSE_BYTES + 1)], &[])
                .unwrap()
                .issues[0]
                .code,
            "compose.too_large"
        );
    }

    #[test]
    fn unsupported_values_and_invalid_nested_structures_fail_closed() {
        let report = analyze_swarm_compatibility(
            &["services:\n  api:\n    image: nginx\n    deploy: replicated\n    ports:\n      target: 80\n"
                .to_owned()],
            &[],
        )
        .unwrap();
        assert!(!report.is_compatible);
        assert!(report.issues.iter().any(|issue| {
            issue.code == "compose.invalid_structure"
                && issue.field_path.as_deref() == Some("services.api.deploy")
        }));
        assert!(report.issues.iter().any(|issue| {
            issue.code == "compose.invalid_structure"
                && issue.field_path.as_deref() == Some("services.api.ports")
        }));

        let report = analyze_swarm_compatibility(
            &["services:\n  api:\n    image: nginx\n    deploy:\n      mode: magic\n".to_owned()],
            &[],
        )
        .unwrap();
        assert!(report.issues.iter().any(|issue| {
            issue.code == "compose.unsupported_value"
                && issue.field_path.as_deref() == Some("services.api.deploy.mode")
        }));
    }

    #[test]
    fn portability_risks_are_warnings_and_do_not_block_preflight() {
        let report = analyze_swarm_compatibility(
            &["services:\n  api:\n    image: nginx\n    volumes:\n      - ./data:/data\n    ports:\n      - target: 80\n        published: 8080\n        mode: host\nvolumes:\n  local-data:\n    driver: local\n"
                .to_owned()],
            &[],
        )
        .unwrap();
        assert!(report.is_compatible);
        assert!(
            report
                .issues
                .iter()
                .all(|issue| { issue.severity == SwarmStackCompatibilitySeverity::Warning })
        );
        for code in [
            "mount.bind_portability",
            "port.host_mode_portability",
            "volume.local_portability",
        ] {
            assert!(report.issues.iter().any(|issue| issue.code == code));
        }
    }
}
