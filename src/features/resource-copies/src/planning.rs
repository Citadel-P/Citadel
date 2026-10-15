use crate::*;
use serde_yaml_ng::Value;
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

pub fn validate_request(request: &CopyRequest) -> Result<(), CopyError> {
    if request.source_id.is_nil()
        || request.name.is_empty()
        || request.name.len() > 64
        || !request
            .name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
    {
        return Err(CopyError::Validation(
            "Choose a name of 1–64 letters, numbers, dots, hyphens or underscores.".into(),
        ));
    }
    if request
        .destination_platform_id
        .is_some_and(|id| id.is_nil())
        || request.destination_node_id.as_ref().is_some_and(|id| {
            id.is_empty() || id.len() > 128 || !id.bytes().all(|b| b.is_ascii_alphanumeric())
        })
    {
        return Err(unsupported(
            "Choose a valid destination Platform and Swarm node.",
        ));
    }
    if request.selections.len() > 100 {
        return Err(CopyError::Validation(
            "A copy supports at most 100 volumes.".into(),
        ));
    }
    Ok(())
}

pub fn plan(
    id: Uuid,
    source: CopySource,
    request: CopyRequest,
    snapshots: Vec<CopySnapshot>,
) -> Result<CopyPlan, CopyError> {
    let mut spec = source.spec.clone();
    let mut blockers = Vec::new();
    let volumes = match storage(&source, &mut spec) {
        Ok(volumes) => volumes,
        Err(CopyError::Validation(message)) => {
            blockers.push(message);
            BTreeMap::new()
        }
        Err(error) => return Err(error),
    };
    if source.name.eq_ignore_ascii_case(&request.name) {
        blockers.push("Choose a different name for the copy.".into());
    }
    if volumes.is_empty() && blockers.is_empty() {
        blockers.push(
            "No declared named volumes were found. Use Configuration only for this resource."
                .into(),
        );
    }
    if volumes.len() > 100 {
        blockers.push("A copy supports at most 100 volumes.".into());
    }
    let current = request.data_mode == CopyDataMode::CurrentData;
    if current && !request.selections.is_empty() {
        return Err(unsupported(
            "Current data copying does not accept backup selections.",
        ));
    }
    if current && !request.consistency_acknowledged {
        blockers.push("Confirm that the source data is ready for a file-level snapshot. Stop or quiesce databases before copying.".into());
    }
    let mut selected_names = BTreeSet::new();
    for selection in &request.selections {
        if !volumes.contains_key(&selection.volume) || !selected_names.insert(&selection.volume) {
            return Err(CopyError::Validation(
                "Each source volume must be selected exactly once.".into(),
            ));
        }
    }
    let mut mapped = Vec::new();
    let mut mapping = BTreeMap::new();
    for (index, (name, node)) in volumes.into_iter().enumerate() {
        let candidates = snapshots
            .iter()
            .filter(|s| s.volume == name && s.docker_node_id == node)
            .cloned()
            .collect::<Vec<_>>();
        let selected = request.selections.iter().find(|s| s.volume == name);
        if !current
            && !selected.is_some_and(|selected| {
                candidates.iter().any(|s| {
                    s.backup_run_id == selected.backup_run_id
                        && s.backup_run_item_id == selected.backup_run_item_id
                })
            })
        {
            blockers.push(format!("Select an available backup for {name}."));
        }
        // Names belong to this review, so a lost submit response is idempotent.
        let target = format!("citadel-copy-{}-{index}", id.simple());
        mapping.insert(name.clone(), target.clone());
        mapped.push(CopyVolume {
            source: name,
            target,
            docker_node_id: node,
            snapshots: candidates,
        });
    }
    if blockers.is_empty() {
        rewrite(&source, &mut spec, &mapping)?;
    }
    let preview = CopyPreview {
        destination_node_id: None,
        id, source_name: source.name.clone(), platform_id: source.platform_id,
        platform_name: source.platform_name.clone(), name: request.name.clone(), volumes: mapped, blockers,
        warnings: vec![
            "The copy stays undeployed. Automatic updates, build redeploys, webhooks and deployment hooks are disabled.".into(),
            "Only the selected volume data is restored. Image data, tmpfs and container writable layers are not copied.".into(),
            "Bindings, networks and external services remain shared references. Review credentials and published ports before applying the copy.".into(),
            if current { "Volumes are copied one at a time, not as an atomic application snapshot. Keep applications quiesced until the copy completes. Temporary snapshots and new volumes require free disk space on the source host.".into() } else { "Snapshots may have different timestamps. Application consistency depends on how each backup was taken.".into() },
        ],
    };
    Ok(CopyPlan {
        destination: None,
        current_volumes: Vec::new(),
        request,
        source,
        preview,
        spec,
    })
}

fn unsupported(message: &str) -> CopyError {
    CopyError::Validation(message.into())
}
fn volume_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
}

fn deployment_mount(mount: &str) -> Result<(&str, &str), CopyError> {
    let (source, rest) = mount.split_once(':').ok_or_else(|| unsupported("Anonymous volume mappings require an explicit named-volume configuration before data copying."))?;
    if !volume_name(source) || !rest.starts_with('/') || rest.contains('$') {
        return Err(unsupported(
            "Bind mounts and interpolated mount paths cannot be copied automatically. Use Configuration only and review their storage separately.",
        ));
    }
    Ok((source, rest))
}

/// Fail closed on syntax whose effective storage cannot be proven. In particular,
/// do not rewrite YAML with regexes or resolve secret-bearing environment values.
fn compose(
    source: &CopySource,
    file: &str,
) -> Result<(Value, BTreeMap<String, String>), CopyError> {
    if source.swarm {
        return Err(unsupported(
            "Swarm Stack data copying is unavailable: per-service node placement must be reviewed. Configuration only remains available.",
        ));
    }
    let mut document: Value =
        serde_yaml_ng::from_str(file).map_err(|_| unsupported("Compose YAML is invalid."))?;
    document
        .apply_merge()
        .map_err(|_| unsupported("Compose YAML merge keys could not be resolved."))?;
    let root = document
        .as_mapping()
        .ok_or_else(|| unsupported("Compose must be a mapping."))?;
    if root.contains_key("include") {
        return Err(unsupported(
            "Compose includes are not supported for data copying.",
        ));
    }
    let services = root
        .get("services")
        .and_then(Value::as_mapping)
        .ok_or_else(|| unsupported("Compose services are required."))?;
    let definitions = root.get("volumes").and_then(Value::as_mapping);
    let project = match &source.spec {
        CopySpec::Stack(spec) => spec
            .common()
            .project_name
            .clone()
            .unwrap_or_else(|| citadel_stacks::normalize_project_name(&source.name, source.id)),
        _ => unreachable!(),
    };
    let mut names = BTreeMap::new();
    for (_, service) in services {
        let service = service
            .as_mapping()
            .ok_or_else(|| unsupported("Compose service must be a mapping."))?;
        if [
            "extends",
            "volumes_from",
            "container_name",
            "devices",
            "build",
        ]
        .iter()
        .any(|key| service.contains_key(*key))
        {
            return Err(unsupported(
                "Data copying does not support Compose extends, volumes_from, container_name, devices or build directives. Review these settings first.",
            ));
        }
        if let Some(mounts) = service.get("volumes") {
            let mounts = mounts
                .as_sequence()
                .ok_or_else(|| unsupported("Compose volumes must be a list."))?;
            for mount in mounts {
                let name = if let Some(short) = mount.as_str() {
                    deployment_mount(short)?.0.to_owned()
                } else {
                    let mount = mount
                        .as_mapping()
                        .ok_or_else(|| unsupported("Unsupported Compose mount."))?;
                    match mount.get("type").and_then(Value::as_str) {
                        Some("tmpfs") => continue,
                        Some("volume") => {}
                        _ => {
                            return Err(unsupported(
                                "Bind mounts cannot be copied automatically. Use Configuration only and review their storage separately.",
                            ));
                        }
                    }
                    mount.get("source").and_then(Value::as_str).filter(|s| volume_name(s)).ok_or_else(|| unsupported("Anonymous or interpolated Compose volume sources are not supported for data copying."))?.to_owned()
                };
                let definition = definitions.and_then(|d| d.get(&name)).ok_or_else(|| {
                    unsupported("Every copied Compose volume must have a top-level declaration.")
                })?;
                let physical = if definition.is_null() {
                    format!("{project}_{name}")
                } else {
                    let definition = definition
                        .as_mapping()
                        .ok_or_else(|| unsupported("Unsupported volume declaration."))?;
                    if definition.contains_key("driver_opts")
                        || definition
                            .get("driver")
                            .and_then(Value::as_str)
                            .is_some_and(|d| d != "local")
                    {
                        return Err(unsupported(
                            "Custom volume drivers and driver options are not supported for data copying.",
                        ));
                    }
                    let external = definition
                        .get("external")
                        .and_then(Value::as_bool)
                        .unwrap_or(false);
                    if definition.get("external").is_some_and(|v| !v.is_bool()) {
                        return Err(unsupported(
                            "Use boolean external volume declarations for data copying.",
                        ));
                    }
                    definition
                        .get("name")
                        .and_then(Value::as_str)
                        .map(str::to_owned)
                        .unwrap_or_else(|| {
                            if external {
                                name.clone()
                            } else {
                                format!("{project}_{name}")
                            }
                        })
                };
                if !volume_name(&physical) {
                    return Err(unsupported(
                        "Interpolated or invalid volume names must be resolved explicitly before copying data.",
                    ));
                }
                names.insert(name, physical);
            }
        }
    }
    Ok((document, names))
}

pub(crate) fn storage(
    source: &CopySource,
    spec: &mut CopySpec,
) -> Result<BTreeMap<String, Option<String>>, CopyError> {
    match spec {
        CopySpec::Deployment(spec) => {
            **spec = spec.clone().for_duplicate();
            spec.volumes
                .as_deref()
                .unwrap_or_default()
                .iter()
                .map(|m| deployment_mount(m).map(|(s, _)| (s.to_owned(), None)))
                .collect()
        }
        CopySpec::Stack(spec) => {
            **spec = spec.clone().for_duplicate();
            let citadel_stacks::StackSpec::WebEditor { compose_file, .. } = spec.as_ref() else {
                return Err(unsupported(
                    "Git Stack data copying requires persistent storage mappings across Git updates and is not available yet. Use Configuration only.",
                ));
            };
            let (_, names) = compose(source, compose_file)?;
            Ok(names.into_values().map(|n| (n, None)).collect())
        }
        CopySpec::SwarmService(spec) => {
            **spec = spec.clone().for_duplicate();
            if spec.scheduling_mode != citadel_swarm_services::SchedulingMode::Replicated
                || spec.replicas != Some(1)
            {
                return Err(unsupported(
                    "Swarm data copying requires one replica pinned to an exact node; global or multi-replica datasets cannot be chosen automatically.",
                ));
            }
            let node = pinned_node(spec).ok_or_else(|| {
                unsupported(
                    "Pin the Swarm Service to one exact node.id before copying node-local volumes.",
                )
            })?;
            let mut volumes = BTreeMap::new();
            for mount in &spec.mounts {
                match mount.kind {
                    citadel_swarm_services::MountKind::Tmpfs => continue,
                    citadel_swarm_services::MountKind::Volume if volume_name(&mount.source) => {
                        volumes.insert(mount.source.clone(), Some(node.to_owned()));
                    }
                    _ => {
                        return Err(unsupported(
                            "Bind mounts cannot be copied automatically. Use Configuration only and review their storage separately.",
                        ));
                    }
                }
            }
            Ok(volumes)
        }
    }
}

fn rewrite(
    source: &CopySource,
    spec: &mut CopySpec,
    mapping: &BTreeMap<String, String>,
) -> Result<(), CopyError> {
    match spec {
        CopySpec::Deployment(spec) => {
            spec.volumes = Some(
                spec.volumes
                    .as_deref()
                    .unwrap_or_default()
                    .iter()
                    .map(|m| {
                        let (name, rest) = deployment_mount(m)?;
                        Ok(format!("{}:{rest}", mapping[name]))
                    })
                    .collect::<Result<_, CopyError>>()?,
            );
            spec.validate().map_err(|e| unsupported(&e.to_string()))?;
        }
        CopySpec::Stack(spec) => {
            let citadel_stacks::StackSpec::WebEditor { compose_file, .. } = spec.as_mut() else {
                unreachable!()
            };
            let (mut document, names) = compose(source, compose_file)?;
            for (logical, physical) in names {
                // External names reference only the newly restored volume. Preserve
                // mount-level options; never preserve external source identities.
                document["volumes"][&logical] = serde_yaml_ng::to_value(
                    serde_json::json!({"name":mapping[&physical],"external":true}),
                )
                .map_err(|e| CopyError::Storage(e.to_string()))?;
            }
            *compose_file = serde_yaml_ng::to_string(&document)
                .map_err(|e| CopyError::Storage(e.to_string()))?;
            spec.validate().map_err(|e| unsupported(&e.to_string()))?;
        }
        CopySpec::SwarmService(spec) => {
            for mount in &mut spec.mounts {
                if mount.kind == citadel_swarm_services::MountKind::Volume {
                    mount.source = mapping[&mount.source].clone();
                }
            }
            spec.validate().map_err(|e| unsupported(&e.to_string()))?;
        }
    }
    Ok(())
}

/// A local volume has meaning only on one explicitly selected Swarm node.
pub fn pinned_node(spec: &citadel_swarm_services::SwarmServiceSpec) -> Option<&str> {
    let mut nodes = spec.placement_constraints.iter().filter_map(|constraint| {
        constraint
            .trim()
            .strip_prefix("node.id")?
            .trim_start()
            .strip_prefix("==")
            .map(str::trim)
    });
    let node = nodes.next()?;
    (!node.is_empty() && nodes.next().is_none()).then_some(node)
}

/// Source and destination identities remain separate throughout a reviewed copy.
pub fn validate_destination(
    source: &CopySource,
    request: &CopyRequest,
    platform: Uuid,
    swarm: bool,
    node: Option<&str>,
) -> Result<(), CopyError> {
    if source.swarm != swarm {
        return Err(unsupported(
            "Copy destinations must have the same platform type as the source: Standalone to Standalone, or Swarm to Swarm.",
        ));
    }
    if swarm && node.is_none() {
        return Err(unsupported(
            "Select an exact destination Swarm node for its local volumes.",
        ));
    }
    if !swarm && node.is_some() {
        return Err(unsupported("Standalone copies cannot select a Swarm node."));
    }
    let source_node = match &source.spec {
        CopySpec::SwarmService(s) => pinned_node(s),
        _ => None,
    };
    if request.data_mode == CopyDataMode::CurrentData
        && (platform != source.platform_id || node != source_node)
    {
        return Err(unsupported(
            "Copy current data is available only on the original platform and node. Select existing backups to copy to another destination.",
        ));
    }
    Ok(())
}
