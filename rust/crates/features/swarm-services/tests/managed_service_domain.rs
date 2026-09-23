use std::collections::BTreeMap;

use citadel_swarm_services::{
    PortPublishMode, SchedulingMode, SwarmServiceImageInfo, SwarmServicePort, SwarmServiceSpec,
    UpdateBehavior,
};
use uuid::Uuid;

fn spec() -> SwarmServiceSpec {
    SwarmServiceSpec {
        image: SwarmServiceImageInfo::External {
            registry_id: Uuid::from_u128(0x100),
            image_tag: "nginx:1.27".to_owned(),
            resolved_digest: Some("sha256:old".to_owned()),
        },
        update_behavior: UpdateBehavior::Notify,
        scheduling_mode: SchedulingMode::Replicated,
        replicas: Some(2),
        command: Vec::new(),
        arguments: Vec::new(),
        environment: vec!["B=2".to_owned(), "A=1".to_owned()],
        labels: BTreeMap::from([("Mixed.Key".to_owned(), "web".to_owned())]),
        user: None,
        working_directory: None,
        health_check: None,
        stop_grace_period_nanoseconds: None,
        ports: vec![SwarmServicePort {
            target_port: 80,
            published_port: Some(8080),
            protocol: "tcp".to_owned(),
            publish_mode: PortPublishMode::Ingress,
        }],
        network_ids: vec!["network-b".to_owned(), "network-a".to_owned()],
        mounts: Vec::new(),
        secrets: Vec::new(),
        configs: Vec::new(),
        resources: None,
        placement_constraints: Vec::new(),
        restart_policy: None,
        update_policy: None,
        webhook: None,
    }
}

#[test]
fn storage_contract_round_trips_existing_pascal_case_json() {
    let expected = spec();
    let value = expected.to_storage_value().unwrap();
    assert_eq!(value["Image"]["$type"], "External");
    assert_eq!(value["Image"]["ImageTag"], "nginx:1.27");
    assert_eq!(value["Labels"]["Mixed.Key"], "web");
    assert_eq!(
        SwarmServiceSpec::from_storage_value(value).unwrap(),
        expected
    );
}

#[test]
fn desired_hash_ignores_resolved_digest_and_webhook() {
    let first = spec();
    let mut changed = first.clone();
    if let SwarmServiceImageInfo::External {
        resolved_digest, ..
    } = &mut changed.image
    {
        *resolved_digest = Some("sha256:new".to_owned());
    }
    changed.webhook = Some(citadel_swarm_services::SwarmServiceWebhookConfig {
        enabled: true,
        provider: "Generic".to_owned(),
        auth_scheme: "BearerToken".to_owned(),
        secret: Some("secret".to_owned()),
        branch_filter: None,
    });
    assert_eq!(first.desired_hash(), changed.desired_hash());
}

#[test]
fn desired_hash_treats_set_like_fields_as_order_independent() {
    let first = spec();
    let mut reordered = first.clone();
    reordered.environment.reverse();
    reordered.network_ids.reverse();

    assert_eq!(first.desired_hash(), reordered.desired_hash());
}

#[test]
fn desired_hash_preserves_command_and_argument_order() {
    let mut first = spec();
    first.command = vec!["sh".to_owned(), "-c".to_owned()];
    first.arguments = vec!["echo first".to_owned(), "echo second".to_owned()];
    let mut reordered = first.clone();
    reordered.command.reverse();
    reordered.arguments.reverse();

    assert_ne!(first.desired_hash(), reordered.desired_hash());
}

#[test]
fn create_drops_unapplied_image_provenance() {
    let created = spec().for_create();
    assert!(matches!(
        created.image,
        SwarmServiceImageInfo::External {
            resolved_digest: None,
            ..
        }
    ));
}

#[test]
fn validation_rejects_reserved_labels_and_duplicate_published_ports() {
    let mut reserved = spec();
    reserved
        .labels
        .insert("com.citadel.managed".to_owned(), "true".to_owned());
    assert!(reserved.validate().is_err());

    let mut duplicate = spec();
    duplicate.ports.push(duplicate.ports[0].clone());
    assert!(duplicate.validate().is_err());
}

#[test]
fn scheduling_mode_and_replica_count_are_consistent() {
    let mut global = spec();
    global.scheduling_mode = SchedulingMode::Global;
    assert!(global.validate().is_err());
    global.replicas = None;
    assert!(global.validate().is_ok());
}
