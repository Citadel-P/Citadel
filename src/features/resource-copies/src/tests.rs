use crate::*;
use serde_json::json;
use uuid::Uuid;

fn source(spec: CopySpec) -> CopySource {
    CopySource {
        kind: match &spec {
            CopySpec::Deployment(_) => CopyKind::Deployment,
            CopySpec::Stack(_) => CopyKind::Stack,
            CopySpec::SwarmService(_) => CopyKind::SwarmService,
        },
        id: Uuid::now_v7(),
        name: "original".into(),
        platform_id: Uuid::now_v7(),
        platform_name: "local".into(),
        swarm: matches!(spec, CopySpec::SwarmService(_)),
        fingerprint: "reviewed-revision".into(),
        spec,
        description: None,
        tag_ids: vec![],
    }
}
fn snapshot(volume: &str, node: Option<&str>) -> CopySnapshot {
    CopySnapshot {
        supports_cross_platform_restore: true,
        backup_run_id: Uuid::now_v7(),
        backup_run_item_id: Uuid::now_v7(),
        policy_id: Uuid::now_v7(),
        policy_name: "daily".into(),
        repository_id: Uuid::now_v7(),
        volume: volume.into(),
        docker_node_id: node.map(str::to_owned),
        completed_at: None,
    }
}
fn request(source: &CopySource, snapshots: &[CopySnapshot]) -> CopyRequest {
    CopyRequest {
        destination_platform_id: None,
        destination_node_id: None,
        data_mode: crate::CopyDataMode::ExistingBackup,
        consistency_acknowledged: false,
        kind: source.kind,
        source_id: source.id,
        name: "independent".into(),
        selections: snapshots
            .iter()
            .map(|s| CopySelection {
                volume: s.volume.clone(),
                backup_run_id: s.backup_run_id,
                backup_run_item_id: s.backup_run_item_id,
            })
            .collect(),
    }
}
fn deployment(mounts: serde_json::Value) -> CopySource {
    source(CopySpec::Deployment(Box::new(
        serde_json::from_value(
            json!({"image":{"$type":"Local","imageId":"test-image"},"volumes":mounts}),
        )
        .unwrap(),
    )))
}
fn stack(yaml: &str) -> CopySource {
    source(CopySpec::Stack(Box::new(
        serde_json::from_value(json!({"$type":"WebEditor","composeFile":yaml})).unwrap(),
    )))
}

#[test]
fn selected_snapshot_is_bound_to_both_run_and_item_and_exact_node() {
    let source = deployment(json!(["source:/data:ro"]));
    let snapshot = snapshot("source", None);
    let mut request = request(&source, std::slice::from_ref(&snapshot));
    request.selections[0].backup_run_id = Uuid::now_v7();
    let plan = planning::plan(
        Uuid::now_v7(),
        source.clone(),
        request,
        vec![snapshot.clone()],
    )
    .unwrap();
    assert!(!plan.preview.blockers.is_empty());
    let request = request_fn(&source, &snapshot);
    let mut wrong_node = snapshot.clone();
    wrong_node.docker_node_id = Some("another-node".into());
    assert!(
        !planning::plan(Uuid::now_v7(), source, request, vec![wrong_node])
            .unwrap()
            .preview
            .blockers
            .is_empty()
    );
}
fn request_fn(source: &CopySource, snapshot: &CopySnapshot) -> CopyRequest {
    request(source, std::slice::from_ref(snapshot))
}

#[test]
fn copied_storage_cannot_follow_an_edited_platform_or_node_placement() {
    let platform = Uuid::now_v7();
    let mut receipt = RestoredCopyVolume {
        platform_id: platform,
        restore_id: Uuid::now_v7(),
        node_id: None,
        name: "copied-data".into(),
    };
    assert!(receipt.verify_location(platform, None).is_ok());
    assert!(receipt.verify_location(Uuid::now_v7(), None).is_err());
    receipt.node_id = Some("original-node".into());
    assert!(
        receipt
            .verify_location(platform, Some("original-node"))
            .is_ok()
    );
    assert!(
        receipt
            .verify_location(platform, Some("another-node"))
            .is_err()
    );
    assert!(receipt.verify_location(platform, None).is_err());
}

#[test]
fn every_distinct_volume_requires_a_snapshot_and_receives_a_distinct_target() {
    let source = deployment(json!(["database:/db", "uploads:/uploads"]));
    let snapshots = vec![snapshot("database", None), snapshot("uploads", None)];
    let incomplete = planning::plan(
        Uuid::now_v7(),
        source.clone(),
        request(&source, &snapshots[..1]),
        snapshots.clone(),
    )
    .unwrap();
    assert_eq!(incomplete.preview.volumes.len(), 2);
    assert_eq!(
        incomplete.preview.blockers,
        ["Select an available backup for uploads."]
    );
    let complete = planning::plan(
        Uuid::now_v7(),
        source.clone(),
        request(&source, &snapshots),
        snapshots,
    )
    .unwrap();
    assert!(complete.preview.blockers.is_empty());
    let targets = complete
        .preview
        .volumes
        .iter()
        .map(|v| &v.target)
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(targets.len(), 2);
    assert!(
        complete
            .preview
            .volumes
            .iter()
            .all(|v| v.source != v.target)
    );
}

#[test]
fn deployment_copy_maps_repeated_mounts_to_one_independent_volume_and_preserves_flags() {
    let source = deployment(json!(["data:/data:ro", "data:/shared:rw"]));
    let snapshot = snapshot("data", None);
    let original = source.spec.clone();
    let plan = planning::plan(
        Uuid::now_v7(),
        source.clone(),
        request_fn(&source, &snapshot),
        vec![snapshot],
    )
    .unwrap();
    assert!(
        plan.preview.blockers.is_empty(),
        "{:?}",
        plan.preview.blockers
    );
    assert_eq!(plan.preview.volumes.len(), 1);
    let CopySpec::Deployment(spec) = plan.spec else {
        panic!()
    };
    let target = &plan.preview.volumes[0].target;
    assert_eq!(
        spec.volumes.unwrap(),
        vec![format!("{target}:/data:ro"), format!("{target}:/shared:rw")]
    );
    assert_eq!(
        serde_json::to_value(original).unwrap(),
        serde_json::to_value(&source.spec).unwrap()
    );
}

#[test]
fn compose_shared_external_volume_is_restored_once_and_both_services_use_new_name() {
    let source = stack(
        "services:\n  a:\n    image: alpine\n    volumes: [data:/a:ro]\n  b:\n    image: alpine\n    volumes:\n      - type: volume\n        source: data\n        target: /b\n        read_only: true\nvolumes:\n  data:\n    external: true\n    name: production-data\n",
    );
    let snapshot = snapshot("production-data", None);
    let plan = planning::plan(
        Uuid::now_v7(),
        source.clone(),
        request_fn(&source, &snapshot),
        vec![snapshot],
    )
    .unwrap();
    assert!(
        plan.preview.blockers.is_empty(),
        "{:?}",
        plan.preview.blockers
    );
    assert_eq!(plan.preview.volumes.len(), 1);
    let CopySpec::Stack(spec) = plan.spec else {
        panic!()
    };
    let citadel_stacks::StackSpec::WebEditor { compose_file, .. } = *spec else {
        panic!()
    };
    let yaml: serde_yaml_ng::Value = serde_yaml_ng::from_str(&compose_file).unwrap();
    assert_eq!(
        yaml["volumes"]["data"]["name"].as_str(),
        Some(plan.preview.volumes[0].target.as_str())
    );
    assert_eq!(
        yaml["services"]["b"]["volumes"][0]["read_only"].as_bool(),
        Some(true)
    );
    assert!(!compose_file.contains("production-data"));
}

#[test]
fn unsupported_storage_and_compose_indirection_are_blocked_without_partial_rewrites() {
    for source in [
        deployment(json!(["/host:/data"])),
        deployment(json!(["/anonymous"])),
        stack("include: elsewhere.yml\nservices: {}\n"),
        stack(
            "services:\n  a:\n    image: alpine\n    volumes: [data:/data]\nvolumes:\n  data:\n    name: ${PRODUCTION_VOLUME}\n",
        ),
        stack(
            "services:\n  a:\n    image: alpine\n    volumes: [data:/data]\nvolumes:\n  data:\n    driver_opts: {device: /host}\n",
        ),
    ] {
        let plan = planning::plan(
            Uuid::now_v7(),
            source.clone(),
            request(&source, &[]),
            vec![],
        )
        .unwrap();
        assert!(!plan.preview.blockers.is_empty());
    }
}

#[test]
fn pinned_single_replica_service_preserves_node_placement_and_rejects_global_data() {
    let mut source=source(CopySpec::SwarmService(Box::new(serde_json::from_value(json!({"image":{"$type":"External","registryId":Uuid::now_v7(),"imageTag":"alpine"},"schedulingMode":"Replicated","replicas":1,"mounts":[{"kind":"Volume","source":"data","target":"/data","readOnly":false}],"placementConstraints":["node.id == abc"]})).unwrap())));
    let snapshot = snapshot("data", Some("abc"));
    let plan = planning::plan(
        Uuid::now_v7(),
        source.clone(),
        request_fn(&source, &snapshot),
        vec![snapshot],
    )
    .unwrap();
    assert!(
        plan.preview.blockers.is_empty(),
        "{:?}",
        plan.preview.blockers
    );
    let CopySpec::SwarmService(spec) = plan.spec else {
        panic!()
    };
    assert_eq!(spec.placement_constraints, vec!["node.id == abc"]);
    if let CopySpec::SwarmService(spec) = &mut source.spec {
        spec.scheduling_mode = citadel_swarm_services::SchedulingMode::Global;
        spec.replicas = None;
    }
    assert!(
        !planning::plan(
            Uuid::now_v7(),
            source.clone(),
            request(&source, &[]),
            vec![]
        )
        .unwrap()
        .preview
        .blockers
        .is_empty()
    );
}

#[test]
fn current_data_needs_no_snapshot_but_requires_consistency_confirmation() {
    for source in [
        deployment(json!(["data:/data"])),
        stack(
            "services:\n  a:\n    image: alpine\n    volumes: [data:/data]\nvolumes:\n  data: {}\n",
        ),
    ] {
        let mut request = request(&source, &[]);
        request.data_mode = CopyDataMode::CurrentData;
        let preview =
            planning::plan(Uuid::now_v7(), source.clone(), request.clone(), vec![]).unwrap();
        assert_eq!(preview.preview.volumes.len(), 1);
        assert_eq!(preview.preview.blockers.len(), 1);
        assert!(preview.preview.blockers[0].contains("file-level snapshot"));
        request.consistency_acknowledged = true;
        let plan = planning::plan(Uuid::now_v7(), source.clone(), request.clone(), vec![]).unwrap();
        assert!(
            plan.preview.blockers.is_empty(),
            "{:?}",
            plan.preview.blockers
        );
        assert!(plan.preview.volumes[0].snapshots.is_empty());
        assert_ne!(
            plan.preview.volumes[0].source,
            plan.preview.volumes[0].target
        );
        assert_ne!(
            serde_json::to_value(plan.spec).unwrap(),
            serde_json::to_value(&source.spec).unwrap()
        );
        request.selections.push(CopySelection {
            volume: "data".into(),
            backup_run_id: Uuid::now_v7(),
            backup_run_item_id: Uuid::now_v7(),
        });
        assert!(planning::plan(Uuid::now_v7(), source, request, vec![]).is_err());
    }
}

#[test]
fn older_copy_requests_keep_existing_backup_semantics() {
    let request: CopyRequest = serde_json::from_value(
        json!({"kind":"Deployment", "sourceId":Uuid::now_v7(), "name":"copy", "selections":[]}),
    )
    .unwrap();
    assert_eq!(request.data_mode, CopyDataMode::ExistingBackup);
    assert!(!request.consistency_acknowledged);
}

#[test]
fn destinations_reject_mixed_platform_types_and_current_data_transfers() {
    let source = deployment(json!(["data:/data"]));
    let mut req = request(&source, &[]);
    let target = Uuid::now_v7();
    assert!(planning::validate_destination(&source, &req, target, false, None).is_ok());
    assert!(planning::validate_destination(&source, &req, target, true, Some("node")).is_err());
    assert!(planning::validate_destination(&source, &req, target, false, Some("node")).is_err());
    req.data_mode = CopyDataMode::CurrentData;
    assert!(planning::validate_destination(&source, &req, target, false, None).is_err());
    assert!(planning::validate_destination(&source, &req, source.platform_id, false, None).is_ok());
}

#[test]
fn swarm_backup_copies_require_a_node_and_current_data_keeps_original_placement() {
    let source=source(CopySpec::SwarmService(Box::new(serde_json::from_value(json!({"image":{"$type":"External","registryId":Uuid::now_v7(),"imageTag":"alpine"},"schedulingMode":"Replicated","replicas":1,"mounts":[{"kind":"Volume","source":"data","target":"/data","readOnly":false}],"placementConstraints":["node.id == abc"]})).unwrap())));
    let mut req = request(&source, &[]);
    let target = Uuid::now_v7();
    assert!(planning::validate_destination(&source, &req, target, true, None).is_err());
    assert!(planning::validate_destination(&source, &req, target, false, None).is_err());
    assert!(planning::validate_destination(&source, &req, target, true, Some("def")).is_ok());
    req.data_mode = CopyDataMode::CurrentData;
    assert!(
        planning::validate_destination(&source, &req, source.platform_id, true, Some("def"))
            .is_err()
    );
    assert!(
        planning::validate_destination(&source, &req, source.platform_id, true, Some("abc"))
            .is_ok()
    );
}

#[test]
fn older_durable_plans_default_to_the_original_platform_and_node() {
    let source = deployment(json!(["data:/data"]));
    let plan = planning::plan(
        Uuid::now_v7(),
        source.clone(),
        request(&source, &[]),
        vec![],
    )
    .unwrap();
    let mut json = serde_json::to_value(&plan).unwrap();
    json.as_object_mut().unwrap().remove("destination");
    let decoded: CopyPlan = serde_json::from_value(json).unwrap();
    assert_eq!(decoded.target_platform(), source.platform_id);
    assert_eq!(decoded.target_node(&decoded.preview.volumes[0]), None);
    assert!(!decoded.moves_storage());
}
