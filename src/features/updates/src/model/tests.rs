use super::*;
use chrono::{Duration, Utc};
use citadel_primitives::ActorId;
use uuid::Uuid;

fn plan() -> UpdatePlan {
    let now = Utc::now();
    let source = CoreIdentity {
        version: "0.1.1".into(),
        image: format!("ghcr.io/citadel-p/citadel@sha256:{}", "a".repeat(64)),
        catalog_sha256: "b".repeat(64),
        architecture: "amd64".into(),
    };
    UpdatePlan {
        id: Uuid::now_v7(),
        instance_id: Uuid::now_v7(),
        target: CoreIdentity {
            version: "0.1.2".into(),
            image: format!("ghcr.io/citadel-p/citadel@sha256:{}", "c".repeat(64)),
            ..source.clone()
        },
        helper_image: source.image.clone(),
        source,
        configuration_sha256: "d".repeat(64),
        registration_id: Uuid::now_v7(),
        reviewed_at: now,
        expires_at: now + Duration::minutes(10),
        checkpoint_directory: "/var/lib/citadel-updates/checkpoints".into(),
        agent_requirements: vec![],
    }
}

fn journal() -> UpdateJournal {
    let plan = plan();
    let operation = UpdateOperation::approve(
        plan.clone(),
        ActorId::new(Uuid::now_v7()),
        Uuid::now_v7(),
        plan.reviewed_at,
    )
    .unwrap();
    UpdateJournal {
        protocol: UPDATE_PROTOCOL,
        history: vec![operation.clone()],
        operation,
        helper_container_id: "qualified-helper".into(),
        verified_target: None,
        completion_acknowledged: false,
        maintenance_released: false,
    }
}
fn advance(journal: &mut UpdateJournal, phase: UpdatePhase) {
    let operation = &mut journal.operation;
    operation
        .advance(
            operation.revision,
            phase,
            operation.updated_at + Duration::seconds(1),
        )
        .unwrap();
    journal.history.push(operation.clone());
}

#[test]
fn approval_rejects_expired_review_moving_images_and_unqualified_helper() {
    let mut plan = plan();
    assert!(plan.validate(plan.expires_at).is_err());
    plan.helper_image = plan.target.image.clone();
    assert!(plan.validate(plan.reviewed_at).is_err());
    plan.helper_image = plan.source.image.clone();
    plan.target.image = "ghcr.io/citadel-p/citadel:latest".into();
    assert!(plan.validate(plan.reviewed_at).is_err());
}

#[test]
fn revision_phase_and_clock_must_advance_together() {
    let mut operation = journal().operation;
    let initial = operation.clone();
    assert!(
        operation
            .advance(1, UpdatePhase::Draining, initial.updated_at)
            .is_err()
    );
    assert!(
        operation
            .advance(0, UpdatePhase::ReplacingCore, initial.updated_at)
            .is_err()
    );
    assert!(
        operation
            .advance(
                0,
                UpdatePhase::Draining,
                initial.updated_at - Duration::seconds(1)
            )
            .is_err()
    );
    assert_eq!(operation, initial);
    operation
        .advance(0, UpdatePhase::Failed, initial.updated_at)
        .unwrap();
    assert!(
        operation
            .advance(1, UpdatePhase::Succeeded, initial.updated_at)
            .is_err()
    );
}

#[test]
fn journal_rejects_missing_checkpoint_and_altered_or_skipped_history() {
    let mut journal = journal();
    let initial = journal.operation.clone();
    advance(&mut journal, UpdatePhase::Draining);
    advance(&mut journal, UpdatePhase::BackingUp);
    advance(&mut journal, UpdatePhase::ReplacingCore);
    assert!(journal.validate_for(&initial).is_err());
    journal.operation.checkpoint = Some("checkpoint-1".into());
    *journal.history.last_mut().unwrap() = journal.operation.clone();
    journal.validate_for(&initial).unwrap();
    let valid = journal.clone();
    journal.history.remove(1);
    assert!(journal.validate_for(&initial).is_err());
    journal = valid.clone();
    journal.operation.actor_id = ActorId::new(Uuid::now_v7());
    assert!(journal.validate_for(&initial).is_err());
    journal = valid;
    journal.history[1].plan.target.image = initial.plan.source.image.clone();
    assert!(journal.validate_for(&initial).is_err());
}

#[test]
fn completion_handshake_never_unlocks_an_uncommitted_or_restored_source() {
    let mut journal = journal();
    advance(&mut journal, UpdatePhase::Draining);
    advance(&mut journal, UpdatePhase::BackingUp);
    journal.operation.checkpoint = Some("checkpoint-1".into());
    advance(&mut journal, UpdatePhase::ReplacingCore);
    advance(&mut journal, UpdatePhase::Verifying);
    advance(&mut journal, UpdatePhase::Succeeded);
    assert!(journal.validate_for(&journal.operation).is_err());
    journal.verified_target = Some(journal.operation.plan.target.clone());
    let receipt = CompletionReceipt {
        instance_id: journal.operation.instance_id,
        operation_id: journal.operation.id,
        fence: journal.operation.fence,
        revision: journal.operation.revision,
        target: journal.operation.plan.target.clone(),
    };
    assert!(receipt.permits_resume(&journal, &receipt.target).is_err());
    journal.completion_acknowledged = true;
    assert!(receipt.permits_resume(&journal, &receipt.target).is_err());
    journal.maintenance_released = true;
    receipt.permits_resume(&journal, &receipt.target).unwrap();
    assert!(
        receipt
            .permits_resume(&journal, &journal.operation.plan.source)
            .is_err()
    );
    let mut stale = receipt.clone();
    stale.fence = Uuid::now_v7();
    assert!(stale.permits_resume(&journal, &receipt.target).is_err());
}

#[test]
fn terminal_outcomes_cannot_be_rewritten_by_a_later_journal() {
    let mut journal = journal();
    advance(&mut journal, UpdatePhase::Postponed);
    let committed = journal.operation.clone();
    journal.operation.phase = UpdatePhase::Failed;
    *journal.history.last_mut().unwrap() = journal.operation.clone();
    assert!(journal.validate_for(&committed).is_err());
}

#[test]
fn stopped_attempts_resume_only_the_committed_unchanged_source_after_cleanup() {
    for phases in [
        vec![UpdatePhase::Failed],
        vec![UpdatePhase::Draining, UpdatePhase::Postponed],
        vec![
            UpdatePhase::Draining,
            UpdatePhase::BackingUp,
            UpdatePhase::Failed,
        ],
    ] {
        let mut journal = journal();
        let initial = journal.operation.clone();
        for phase in phases {
            advance(&mut journal, phase);
        }
        let committed = journal.operation.clone();
        let source = committed.plan.source.clone();
        assert!(journal.permits_source_resume(&committed, &source).is_err());
        journal.maintenance_released = true;
        assert!(journal.permits_source_resume(&initial, &source).is_err());
        journal.permits_source_resume(&committed, &source).unwrap();

        for field in ["version", "image", "catalog", "architecture"] {
            let mut changed = source.clone();
            match field {
                "version" => changed.version = "0.1.0".into(),
                "image" => changed.image = committed.plan.target.image.clone(),
                "catalog" => changed.catalog_sha256 = "f".repeat(64),
                "architecture" => changed.architecture = "arm64".into(),
                _ => unreachable!(),
            }
            assert!(
                journal.permits_source_resume(&committed, &changed).is_err(),
                "{field}"
            );
        }
        let mut conflicting = journal.clone();
        conflicting.completion_acknowledged = true;
        assert!(
            conflicting
                .permits_source_resume(&committed, &source)
                .is_err()
        );
        conflicting = journal.clone();
        conflicting.verified_target = Some(committed.plan.target.clone());
        assert!(
            conflicting
                .permits_source_resume(&committed, &source)
                .is_err()
        );
        conflicting = journal;
        conflicting.history.remove(0);
        assert!(
            conflicting
                .permits_source_resume(&committed, &source)
                .is_err()
        );
    }
}

#[test]
fn source_resume_never_releases_active_or_ambiguous_replacement() {
    let mut journal = journal();
    for phase in [
        UpdatePhase::Preparing,
        UpdatePhase::Draining,
        UpdatePhase::BackingUp,
        UpdatePhase::ReplacingCore,
        UpdatePhase::RecoveryRequired,
    ] {
        if phase != UpdatePhase::Preparing {
            if phase == UpdatePhase::ReplacingCore {
                journal.operation.checkpoint = Some("checkpoint-1".into());
            }
            advance(&mut journal, phase);
        }
        journal.maintenance_released = true;
        assert!(
            journal
                .permits_source_resume(&journal.operation, &journal.operation.plan.source)
                .is_err()
        );
        journal.maintenance_released = false;
    }
    // Relabelling a replacement failure does not create a safe source outcome.
    journal.operation.phase = UpdatePhase::Failed;
    *journal.history.last_mut().unwrap() = journal.operation.clone();
    journal.maintenance_released = true;
    assert!(
        journal
            .permits_source_resume(&journal.operation, &journal.operation.plan.source)
            .is_err()
    );
}
