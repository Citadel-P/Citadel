use chrono::{Duration, Utc};
use citadel_activities::{
    ActivityAccess, ActivityFilter, ActivityQueryStore, ActivityResourceType,
};
use citadel_adapters::persistence::postgres::updates::check_startup_updates;
use citadel_adapters::persistence::postgres::{
    activities::store::PostgresActivityStore,
    identity::authentication::store::PostgresIdentityStore, updates::PostgresUpdateRepository,
};
use citadel_database::MigrationRunner;
use citadel_identity::{
    ADMIN_ROLE_ID, ActorPrincipal, AuthenticatedPrincipalType, IdentityStore, MfaPolicy,
    PatchInstallationSettings, SYSTEM_ACTOR_ID,
};
use citadel_primitives::{ActorId, PatchField};
use citadel_updates::*;
use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;

#[path = "instance_updates/acceptance.rs"]
mod acceptance;
#[path = "instance_updates/drain.rs"]
mod drain;

#[tokio::test]
#[ignore = "requires a fresh disposable CITADEL_UPDATE_TEST_DATABASE_URL (database name citadel_update_test_*)"]
async fn update_state_activities_and_receipts_commit_atomically() {
    let url = std::env::var("CITADEL_UPDATE_TEST_DATABASE_URL").unwrap();
    assert!(
        url::Url::parse(&url)
            .unwrap()
            .path()
            .starts_with("/citadel_update_test_")
    );
    let pool = PgPoolOptions::new()
        .max_connections(6)
        .connect(&url)
        .await
        .unwrap();
    assert!(
        sqlx::query_scalar::<_, bool>("SELECT to_regclass('public.users') IS NULL")
            .fetch_one(&pool)
            .await
            .unwrap(),
        "fixture requires a fresh database"
    );
    check_startup_updates(&pool, None).await.unwrap();
    assert!(
        check_startup_updates(&pool, Some(Uuid::now_v7()))
            .await
            .is_err()
    );
    MigrationRunner::migrate(&url).await.unwrap();
    check_startup_updates(&pool, None).await.unwrap();
    use citadel_adapters::persistence::postgres::updates::read_instance_identity;
    assert!(read_instance_identity(&pool).await.is_err());
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM citadelinstanceidentity")
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
    let actor = UpdateActor {
        subject_id: Uuid::now_v7(),
        actor_id: ActorId::new(Uuid::now_v7()),
    };
    let instance = Uuid::now_v7();
    sqlx::query("INSERT INTO actors(id,isenabled,type) VALUES($1,TRUE,'User')")
        .bind(actor.actor_id.value())
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO users(id,actorid,createdat,createdbyactorid,email,name,password) VALUES($1,$2,CURRENT_TIMESTAMP,$3,'update-test@example.test','update-test',NULL)").bind(actor.subject_id).bind(actor.actor_id.value()).bind(SYSTEM_ACTOR_ID).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO actorroles(actorid,roleid) VALUES($1,$2)")
        .bind(actor.actor_id.value())
        .bind(ADMIN_ROLE_ID)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO citadelinstanceidentity(id,instanceid,createdat) VALUES(1,$1,CURRENT_TIMESTAMP)").bind(instance).execute(&pool).await.unwrap();
    assert!(read_instance_identity(&pool).await.is_err());
    sqlx::query("UPDATE instancesetupstates SET initializedat=CURRENT_TIMESTAMP,initialadministratoractorid=$1 WHERE id=1").bind(actor.actor_id.value()).execute(&pool).await.unwrap();
    assert_eq!(read_instance_identity(&pool).await.unwrap(), instance);
    check_startup_updates(&pool, Some(instance)).await.unwrap();
    assert!(
        check_startup_updates(&pool, Some(Uuid::now_v7()))
            .await
            .is_err()
    );
    let repo = PostgresUpdateRepository::new(pool.clone());
    let release = ReleaseNotice {
        version: "0.1.2".into(),
        release_url: "https://github.com/Citadel-P/Citadel/releases/tag/v0.1.2".into(),
    };
    let checks = futures_util::future::join_all(
        (0..12).map(|_| repo.record_check(Ok(Some(&release)), Utc::now())),
    )
    .await;
    assert_eq!(
        checks.into_iter().filter(|r| *r.as_ref().unwrap()).count(),
        1
    );
    assert_eq!(count(&pool, "UpdateDiscovered").await, 1);
    repo.record_check(
        Err("must not persist credentials or transport output"),
        Utc::now(),
    )
    .await
    .unwrap();
    let status = repo.checks().await.unwrap();
    assert_eq!(status.available, Some(release.clone()));
    assert!(status.last_success.is_some());
    assert!(!status.error.unwrap().contains("credentials"));

    let now = Utc::now();
    let source = CoreIdentity {
        version: "0.1.1".into(),
        image: format!("ghcr.io/citadel-p/citadel@sha256:{}", "a".repeat(64)),
        catalog_sha256: MigrationRunner::catalog_sha256().unwrap(),
        architecture: "amd64".into(),
    };
    let plan = UpdatePlan {
        id: Uuid::now_v7(),
        instance_id: instance,
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
        checkpoint_directory: "/private/fixture/checkpoints".into(),
        agent_requirements: vec![],
    };
    repo.save_review(actor, &plan).await.unwrap();
    sqlx::query("UPDATE actors SET isenabled=FALSE WHERE id=$1")
        .bind(actor.actor_id.value())
        .execute(&pool)
        .await
        .unwrap();
    assert!(matches!(
        repo.claim(actor, &plan, Uuid::now_v7(), now).await,
        Err(UpdateError::Forbidden)
    ));
    sqlx::query("UPDATE actors SET isenabled=TRUE WHERE id=$1")
        .bind(actor.actor_id.value())
        .execute(&pool)
        .await
        .unwrap();

    sqlx::raw_sql("CREATE FUNCTION reject_update_activity() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.eventtype='InstanceUpdateStarted' THEN RAISE EXCEPTION 'fixture activity failure'; END IF; RETURN NEW; END $$; CREATE TRIGGER reject_update_activity BEFORE INSERT ON activityevents FOR EACH ROW EXECUTE FUNCTION reject_update_activity();").execute(&pool).await.unwrap();
    assert!(repo.claim(actor, &plan, Uuid::now_v7(), now).await.is_err());
    assert!(repo.active().await.unwrap().is_none());
    assert_eq!(count(&pool, "InstanceUpdateStarted").await, 0);
    sqlx::raw_sql("DROP TRIGGER reject_update_activity ON activityevents; DROP FUNCTION reject_update_activity();").execute(&pool).await.unwrap();
    let key = Uuid::now_v7();
    let claims =
        futures_util::future::join_all((0..8).map(|_| repo.claim(actor, &plan, key, now))).await;
    let operation = claims[0].as_ref().unwrap().clone();
    assert!(check_startup_updates(&pool, None).await.is_err());
    for claim in claims {
        assert_eq!(claim.unwrap(), operation);
    }
    assert!(repo.claim(actor, &plan, Uuid::now_v7(), now).await.is_err());
    assert_eq!(count(&pool, "InstanceUpdateStarted").await, 1);
    drain::verify_owned_drain_observations(&pool, &repo, &operation).await;
    let mut journal = UpdateJournal {
        protocol: UPDATE_PROTOCOL,
        history: vec![operation.clone()],
        operation,
        helper_container_id: "fixture-helper".into(),
        verified_target: None,
        completion_acknowledged: false,
        maintenance_released: false,
    };
    for phase in [
        UpdatePhase::Draining,
        UpdatePhase::BackingUp,
        UpdatePhase::ReplacingCore,
        UpdatePhase::Verifying,
        UpdatePhase::Succeeded,
    ] {
        if phase == UpdatePhase::ReplacingCore {
            journal.operation.checkpoint = Some("fixture-checkpoint".into());
        }
        journal
            .operation
            .advance(journal.operation.revision, phase, Utc::now())
            .unwrap();
        journal.history.push(journal.operation.clone());
        if phase == UpdatePhase::Verifying {
            acceptance::verify_target_acceptance(&pool, &journal, actor.subject_id).await;
        }
    }
    assert!(repo.import(&journal).await.is_err());
    journal.verified_target = Some(plan.target.clone());
    let uncommitted = CompletionReceipt {
        instance_id: journal.operation.instance_id,
        operation_id: journal.operation.id,
        fence: journal.operation.fence,
        revision: journal.operation.revision,
        target: plan.target.clone(),
    };
    assert!(
        citadel_adapters::persistence::postgres::updates::check_committed_update_receipt(
            &pool,
            &journal,
            &uncommitted
        )
        .await
        .is_err()
    );
    let receipt = repo.import(&journal).await.unwrap().unwrap();
    citadel_adapters::persistence::postgres::updates::check_committed_update_receipt(
        &pool, &journal, &receipt,
    )
    .await
    .unwrap();
    let mut foreign_receipt = receipt.clone();
    foreign_receipt.fence = Uuid::now_v7();
    assert!(
        citadel_adapters::persistence::postgres::updates::check_committed_update_receipt(
            &pool,
            &journal,
            &foreign_receipt
        )
        .await
        .is_err()
    );
    assert!(
        repo.blockers(&journal.operation).await.is_err(),
        "a completed update must not enter drain again"
    );
    assert!(
        check_startup_updates(&pool, Some(instance)).await.is_err(),
        "terminal but unreleased ownership still blocks startup"
    );
    assert_eq!(repo.import(&journal).await.unwrap(), Some(receipt.clone()));
    assert_eq!(count(&pool, "InstanceUpdateCompleted").await, 1);
    assert!(repo.release(&journal, &plan.target).await.is_err());
    journal.completion_acknowledged = true;
    journal.maintenance_released = true;
    assert!(repo.release(&journal, &plan.source).await.is_err());
    repo.release(&journal, &plan.target).await.unwrap();
    repo.release(&journal, &plan.target).await.unwrap();
    assert!(repo.active().await.unwrap().is_none());
    let mut completed_only = journal.clone();
    completed_only.completion_acknowledged = false;
    completed_only.maintenance_released = false;
    assert!(
        citadel_adapters::persistence::postgres::updates::check_committed_update_receipt(
            &pool,
            &completed_only,
            &receipt
        )
        .await
        .is_err()
    );

    let identity = PostgresIdentityStore::new(pool.clone());
    check_startup_updates(&pool, Some(instance)).await.unwrap();
    let principal = ActorPrincipal {
        subject_id: actor.subject_id,
        actor_id: actor.actor_id,
        name: "update-test".into(),
        principal_type: AuthenticatedPrincipalType::User,
        credential_id: None,
        roles: vec!["Admin".into()],
    };
    let patch = PatchInstallationSettings {
        update_checks_enabled: PatchField::Value(false),
        update_check_interval_hours: PatchField::Value(12),
        mfa_policy: PatchField::Value(MfaPolicy::RequiredForAdministrators),
    };
    identity
        .patch_installation_settings(&principal, &patch, Utc::now())
        .await
        .unwrap();
    identity
        .patch_installation_settings(&principal, &patch, Utc::now())
        .await
        .unwrap();
    assert_eq!(count(&pool, "UpdateChecksChanged").await, 1);
    assert_eq!(count(&pool, "InstallationSettingsUpdated").await, 1);
    assert_eq!(
        identity
            .installation_settings()
            .await
            .unwrap()
            .update_check_interval_hours,
        12
    );
    let frequency = PatchInstallationSettings {
        update_check_interval_hours: PatchField::Value(24),
        ..Default::default()
    };
    for _ in 0..2 {
        identity
            .patch_installation_settings(&principal, &frequency, Utc::now())
            .await
            .unwrap();
    }
    assert_eq!(
        count(&pool, "UpdateChecksChanged").await,
        2,
        "frequency-only edits must record one activity"
    );
    assert_eq!(
        count(&pool, "InstallationSettingsUpdated").await,
        1,
        "frequency must not create MFA history"
    );
    let settings = identity.installation_settings().await.unwrap();
    assert_eq!(settings.update_check_interval_hours, 24);
    assert!(
        !settings.update_checks_enabled,
        "changing frequency must not enable checks"
    );
    let info: serde_json::Value = sqlx::query_scalar("SELECT info::jsonb FROM activityevents WHERE eventtype='UpdateChecksChanged' AND info::jsonb->>'UpdateCheckIntervalHours'='24'")
        .fetch_one(&pool).await.unwrap();
    assert_eq!(info["PreviousUpdateCheckIntervalHours"], 12);
    let info: serde_json::Value = sqlx::query_scalar(
        "SELECT info::jsonb FROM activityevents WHERE eventtype='InstallationSettingsUpdated'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        info["PreviousUpdateChecksEnabled"],
        info["UpdateChecksEnabled"]
    );
    let store = PostgresActivityStore::new(pool.clone());
    let scope = ActivityFilter {
        resource_type: Some(ActivityResourceType::Updates),
        ..Default::default()
    }
    .validated()
    .unwrap();
    let denied = ActivityAccess {
        actor_id: actor.actor_id,
        administrator: false,
    };
    assert_eq!(
        store
            .list_authorized(&denied, scope)
            .await
            .unwrap()
            .total_count,
        0
    );
    let id: Uuid = sqlx::query_scalar(
        "SELECT id FROM activityevents WHERE eventtype='InstanceUpdateCompleted'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(store.get_authorized(&denied, id).await.unwrap().is_none());
    let allowed = ActivityAccess {
        administrator: true,
        ..denied
    };
    assert!(store.get_authorized(&allowed, id).await.unwrap().is_some());
    let attributed: Uuid =
        sqlx::query_scalar("SELECT createdbyactorid FROM activityevents WHERE id=$1")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(attributed, actor.actor_id.value());

    let mut previous_stopped = None;
    for phase in [UpdatePhase::Postponed, UpdatePhase::Failed] {
        let operation = repo
            .claim(actor, &plan, Uuid::now_v7(), Utc::now())
            .await
            .unwrap();
        if let Some(previous) = &previous_stopped {
            // Replaying an old release must not release the new attempt's owner.
            repo.release(previous, &plan.source).await.unwrap();
            assert_eq!(repo.active().await.unwrap(), Some(operation.clone()));
        }
        let mut stopped = UpdateJournal {
            protocol: UPDATE_PROTOCOL,
            history: vec![operation.clone()],
            operation,
            helper_container_id: "fixture-helper".into(),
            verified_target: None,
            completion_acknowledged: false,
            maintenance_released: false,
        };
        stopped.operation.advance(0, phase, Utc::now()).unwrap();
        stopped.history.push(stopped.operation.clone());
        stopped.maintenance_released = true;
        assert!(repo.release(&stopped, &plan.source).await.is_err());
        stopped.maintenance_released = false;
        assert!(repo.import(&stopped).await.unwrap().is_none());
        assert!(repo.import(&stopped).await.unwrap().is_none());
        assert!(repo.release(&stopped, &plan.source).await.is_err());
        stopped.maintenance_released = true;
        assert!(repo.release(&stopped, &plan.target).await.is_err());
        assert!(
            repo.claim(actor, &plan, Uuid::now_v7(), Utc::now())
                .await
                .is_err()
        );
        assert_eq!(
            repo.active().await.unwrap(),
            Some(stopped.operation.clone())
        );
        repo.release(&stopped, &plan.source).await.unwrap();
        repo.release(&stopped, &plan.source).await.unwrap();
        assert!(repo.active().await.unwrap().is_none());
        previous_stopped = Some(stopped);
    }
    assert_eq!(count(&pool, "InstanceUpdatePostponed").await, 1);
    assert_eq!(count(&pool, "InstanceUpdateFailed").await, 1);
    assert_eq!(count(&pool, "InstanceUpdateCompleted").await, 1);

    let operation = repo
        .claim(actor, &plan, Uuid::now_v7(), Utc::now())
        .await
        .unwrap();
    let mut recovery = UpdateJournal {
        protocol: UPDATE_PROTOCOL,
        history: vec![operation.clone()],
        operation,
        helper_container_id: "fixture-helper".into(),
        verified_target: None,
        completion_acknowledged: false,
        maintenance_released: false,
    };
    for phase in [
        UpdatePhase::Draining,
        UpdatePhase::BackingUp,
        UpdatePhase::ReplacingCore,
        UpdatePhase::RecoveryRequired,
    ] {
        if phase == UpdatePhase::ReplacingCore {
            recovery.operation.checkpoint = Some("fixture-checkpoint".into());
        }
        recovery
            .operation
            .advance(recovery.operation.revision, phase, Utc::now())
            .unwrap();
        recovery.history.push(recovery.operation.clone());
    }
    repo.import(&recovery).await.unwrap();
    assert!(repo.release(&recovery, &plan.source).await.is_err());
    recovery.maintenance_released = true;
    assert!(repo.release(&recovery, &plan.source).await.is_err());
    assert!(
        repo.claim(actor, &plan, Uuid::now_v7(), Utc::now())
            .await
            .is_err()
    );
    assert_eq!(repo.active().await.unwrap(), Some(recovery.operation));
    assert!(check_startup_updates(&pool, None).await.is_err());
    // Even unreadable journal JSON must not bypass retained ownership.
    sqlx::query("UPDATE instanceupdates SET operation='{}'::jsonb WHERE ownsinstallation")
        .execute(&pool)
        .await
        .unwrap();
    assert!(check_startup_updates(&pool, None).await.is_err());
    // A missing table after a completed migration is corruption, not a new install.
    sqlx::query("ALTER TABLE instanceupdates RENAME TO instanceupdates_fixture_missing")
        .execute(&pool)
        .await
        .unwrap();
    assert!(check_startup_updates(&pool, None).await.is_err());
    pool.close().await;
}

async fn count(pool: &sqlx::PgPool, event: &str) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM activityevents WHERE eventtype=$1")
        .bind(event)
        .fetch_one(pool)
        .await
        .unwrap()
}
