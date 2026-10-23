use super::*;
use chrono::DateTime;

fn update_info(kind: &str, target: &str) -> serde_json::Value {
    match kind {
        "DeploymentImageUpdateAvailable" => json!({"LatestImage":target}),
        "StackImageUpdateAvailable" => super::target(target, target),
        "StackGitUpdateAvailable" => json!({"RemoteCommitSha":target}),
        _ => unreachable!(),
    }
}

async fn assert_superseded(f: &Fixture, old: Uuid, replacement: Uuid) {
    let event = f.alerts.get_event(old).await.unwrap();
    assert_eq!(event.status, citadel_alerts::AlertEventStatus::Resolved);
    assert_eq!(event.resolved_by_actor_id, Some(Uuid::from_u128(1)));
    assert_eq!(event.actor_name.as_deref(), Some("System"));
    assert_eq!(
        event.resolution_note,
        Some(format!(
            "Superseded by a newer available update (alert {replacement})."
        ))
    );
    let key: Option<String> =
        sqlx::query_scalar("SELECT openincidentkey FROM alertevents WHERE id=$1")
            .bind(old)
            .fetch_one(&f.pool)
            .await
            .unwrap();
    assert!(key.is_none());
}

#[tokio::test]
#[ignore = "requires CITADEL_EXECUTION_DATABASE_URL"]
async fn unchanged_update_rechecks_fence_delayed_targets_across_restart_without_extending_cooldown()
{
    let f = Fixture::new().await;
    for kind in [
        "DeploymentImageUpdateAvailable",
        "StackImageUpdateAvailable",
        "StackGitUpdateAvailable",
    ] {
        let resource = Uuid::now_v7();
        let rule = f.rule(resource, kind).await;
        let (old, first) = f.event(resource, kind, "a", update_info(kind, "a")).await;
        f.alerts.acknowledge(f.actor, &[old.id]).await.unwrap();
        let acknowledged = f.alerts.get_event(old.id).await.unwrap();
        f.alerts
            .update_rule(f.actor, rule, &json!({"cooldownSeconds":60}))
            .await
            .unwrap();
        let repeat = AlertObservation {
            observed_at: first.observed_at + Duration::seconds(120),
            ..first.clone()
        };
        assert!(f.alerts.process_event(&repeat).await.unwrap().is_none());

        // A new repository instance must retain the fence even though the last
        // observation neither created an event nor extended notification cooldown.
        let resumed = PostgresAlertRepository::new(f.pool.clone())
            .with_entitlements(Arc::new(StaticEntitlementService::new(true)));
        let delayed = AlertObservation {
            deduplication_component: "b".into(),
            info: update_info(kind, "b"),
            observed_at: first.observed_at + Duration::seconds(90),
            ..first.clone()
        };
        assert!(resumed.process_event(&delayed).await.unwrap().is_none());
        let retained = resumed.get_event(old.id).await.unwrap();
        assert_eq!(
            retained.status,
            citadel_alerts::AlertEventStatus::Acknowledged
        );
        assert_eq!(retained.acknowledged_at, acknowledged.acknowledged_at);
        assert_eq!(retained.updated_at, acknowledged.updated_at);
        let (triggered, observed): (DateTime<Utc>, DateTime<Utc>) = sqlx::query_as(
            "SELECT lasttriggeredat,lastobservedat FROM alertrulestates WHERE alertruleid=$1 AND resourceid=$2"
        ).bind(rule).bind(resource).fetch_one(&f.pool).await.unwrap();
        assert_eq!(
            triggered.timestamp_micros(),
            first.observed_at.timestamp_micros()
        );
        assert_eq!(
            observed.timestamp_micros(),
            repeat.observed_at.timestamp_micros()
        );

        let latest = AlertObservation {
            observed_at: repeat.observed_at + Duration::seconds(1),
            ..delayed.clone()
        };
        let replacement = resumed
            .process_event(&latest)
            .await
            .unwrap()
            .expect("unchanged rechecks must not postpone an otherwise eligible replacement");
        assert_superseded(&f, old.id, replacement.id).await;

        // Observations suppressed by cooldown must also reject older targets
        // after the rule's cooldown is removed.
        let suppressed = AlertObservation {
            observed_at: latest.observed_at + Duration::seconds(30),
            deduplication_component: "c".into(),
            info: update_info(kind, "c"),
            ..latest.clone()
        };
        assert!(resumed.process_event(&suppressed).await.unwrap().is_none());
        resumed
            .update_rule(f.actor, rule, &json!({"cooldownSeconds":null}))
            .await
            .unwrap();
        let stale = AlertObservation {
            observed_at: latest.observed_at + Duration::seconds(20),
            ..first.clone()
        };
        assert!(resumed.process_event(&stale).await.unwrap().is_none());
        f.resolved(replacement.id, false).await;
        let eligible = AlertObservation {
            observed_at: suppressed.observed_at + Duration::seconds(1),
            ..suppressed
        };
        assert!(resumed.process_event(&eligible).await.unwrap().is_some());
    }
    f.pool.close().await;
}

#[tokio::test]
#[ignore = "requires CITADEL_EXECUTION_DATABASE_URL"]
async fn update_targets_supersede_atomically_after_cooldown_and_preserve_acknowledgments() {
    let f = Fixture::new().await;
    for kind in [
        "DeploymentImageUpdateAvailable",
        "StackImageUpdateAvailable",
        "StackGitUpdateAvailable",
    ] {
        let resource = Uuid::now_v7();
        let rule = f.rule(resource, kind).await;
        let (old, observation) = f
            .event(resource, kind, "old", update_info(kind, "old"))
            .await;
        f.alerts.acknowledge(f.actor, &[old.id]).await.unwrap();
        f.alerts
            .update_rule(f.actor, rule, &json!({"cooldownSeconds":60}))
            .await
            .unwrap();
        let next = AlertObservation {
            deduplication_component: "new".into(),
            info: update_info(kind, "new"),
            observed_at: observation.observed_at + Duration::seconds(30),
            ..observation.clone()
        };
        let before = f.notifications.load(Ordering::SeqCst);
        assert!(f.alerts.process_event(&next).await.unwrap().is_none());
        f.resolved(old.id, false).await;
        assert_eq!(f.notifications.load(Ordering::SeqCst), before);

        // Unchanged observations after cooldown keep the acknowledged incident.
        let repeat = AlertObservation {
            observed_at: observation.observed_at + Duration::seconds(61),
            ..observation.clone()
        };
        assert!(f.alerts.process_event(&repeat).await.unwrap().is_none());
        assert_eq!(f.notifications.load(Ordering::SeqCst), before);
        assert_eq!(
            f.alerts.get_event(old.id).await.unwrap().status,
            citadel_alerts::AlertEventStatus::Acknowledged
        );

        let next = AlertObservation {
            observed_at: observation.observed_at + Duration::seconds(62),
            ..next
        };
        let (left, right) =
            tokio::join!(f.alerts.process_event(&next), f.alerts.process_event(&next));
        let emitted = [left.unwrap(), right.unwrap()];
        assert_eq!(emitted.iter().flatten().count(), 1);
        let new = emitted.into_iter().flatten().next().unwrap();
        assert_superseded(&f, old.id, new.id).await;
        let closed = f.alerts.get_event(old.id).await.unwrap();
        assert_eq!(closed.acknowledged_by_actor_id, Some(f.actor.value()));
        assert_eq!(new.status, citadel_alerts::AlertEventStatus::Active);
        assert!(new.acknowledged_at.is_none());
        assert_eq!(f.notifications.load(Ordering::SeqCst), before + 1);

        // Removing cooldown must not let an out-of-order observation replace it.
        f.alerts
            .update_rule(f.actor, rule, &json!({"cooldownSeconds":null}))
            .await
            .unwrap();
        assert!(
            f.alerts
                .process_event(&observation)
                .await
                .unwrap()
                .is_none()
        );
        f.resolved(new.id, false).await;
        assert_eq!(f.notifications.load(Ordering::SeqCst), before + 1);

        // Different concurrent targets must converge on the newest observation,
        // regardless of which transaction acquires the rule/resource lock first.
        let middle = AlertObservation {
            deduplication_component: "middle".into(),
            info: update_info(kind, "middle"),
            observed_at: next.observed_at + Duration::seconds(1),
            ..next.clone()
        };
        let latest = AlertObservation {
            deduplication_component: "latest".into(),
            info: update_info(kind, "latest"),
            observed_at: next.observed_at + Duration::seconds(2),
            ..next.clone()
        };
        let (left, right) = tokio::join!(
            f.alerts.process_event(&middle),
            f.alerts.process_event(&latest)
        );
        left.unwrap();
        right.unwrap();
        let open = f
            .alerts
            .list_events(
                f.actor,
                true,
                &AlertEventFilter {
                    resource_id: Some(resource),
                    unresolved_only: true,
                    page_size: 50,
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        assert_eq!(open.items.len(), 1);
        assert_eq!(open.total_count, 1);
        assert_eq!(open.items[0].info, latest.info);
    }
    f.pool.close().await;
}

#[tokio::test]
#[ignore = "requires CITADEL_EXECUTION_DATABASE_URL"]
async fn update_recheck_repairs_existing_open_targets_without_replacing_current_or_other_incidents()
{
    let f = Fixture::new().await;
    let resource = Uuid::now_v7();
    let kind = "DeploymentImageUpdateAvailable";
    f.rule(resource, kind).await;
    let (old, _) = f
        .event(resource, kind, "old", update_info(kind, "old"))
        .await;
    let (current, observation) = f
        .historical_event(&old, "current", update_info(kind, "current"))
        .await;
    let (another, _) = f
        .historical_event(&old, "another", update_info(kind, "another"))
        .await;
    f.alerts.acknowledge(f.actor, &[current.id]).await.unwrap();
    let before_current = f.alerts.get_event(current.id).await.unwrap();

    let other_resource = Uuid::now_v7();
    f.rule(other_resource, kind).await;
    let (unrelated, _) = f
        .event(other_resource, kind, "old", update_info(kind, "old"))
        .await;
    f.rule(resource, "DeploymentAutoDeployFailed").await;
    let (failure, _) = f
        .event(resource, "DeploymentAutoDeployFailed", "failure", json!({}))
        .await;
    let other_rule = f.rule(resource, kind).await;
    let mut other_rule_event = old.clone();
    other_rule_event.alert_rule_id = other_rule;
    let (separate_rule, _) = f
        .historical_event(
            &other_rule_event,
            "other-rule",
            update_info(kind, "other-rule"),
        )
        .await;
    // Keep rule selection deterministic while checking rule-scoped housekeeping.
    f.alerts
        .update_rule(f.actor, other_rule, &json!({"status":"Disabled"}))
        .await
        .unwrap();

    let before = f.notifications.load(Ordering::SeqCst);
    assert!(
        f.alerts
            .process_event(&observation)
            .await
            .unwrap()
            .is_none()
    );
    assert_superseded(&f, old.id, current.id).await;
    assert_superseded(&f, another.id, current.id).await;
    let retained = f.alerts.get_event(current.id).await.unwrap();
    assert_eq!(
        retained.status,
        citadel_alerts::AlertEventStatus::Acknowledged
    );
    assert_eq!(retained.acknowledged_at, before_current.acknowledged_at);
    assert_eq!(retained.updated_at, before_current.updated_at);
    for id in [unrelated.id, failure.id, separate_rule.id] {
        f.resolved(id, false).await;
    }
    assert_eq!(f.notifications.load(Ordering::SeqCst), before + 1);
    assert!(
        f.alerts
            .process_event(&observation)
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(f.notifications.load(Ordering::SeqCst), before + 1);
    f.pool.close().await;
}

#[tokio::test]
#[ignore = "requires CITADEL_EXECUTION_DATABASE_URL"]
async fn update_supersession_rolls_back_with_delivery_failure_and_retries_durably() {
    use citadel_adapters::persistence::postgres::alerts::observations::enqueue;
    let f = Fixture::new().await;
    let resource = Uuid::now_v7();
    let kind = "DeploymentImageUpdateAvailable";
    let rule = f.rule(resource, kind).await;
    let channel = f
        .alerts
        .create_channel(
            f.actor,
            &AlertChannelConfiguration {
                name: format!("supersession-{resource}"),
                alert_destination: "Generic".into(),
                url: "https://alerts.example.test/hook".into(),
                is_active: true,
            },
        )
        .await
        .unwrap();
    f.alerts
        .update_rule(f.actor, rule, &json!({"channelIds":[channel.id]}))
        .await
        .unwrap();
    let (old, observation) = f
        .event(resource, kind, "old", update_info(kind, "old"))
        .await;
    let next = AlertObservation {
        deduplication_component: "new".into(),
        info: update_info(kind, "new"),
        observed_at: observation.observed_at + Duration::seconds(1),
        ..observation.clone()
    };
    let mut tx = f.pool.begin().await.unwrap();
    enqueue(&mut tx, &next).await.unwrap();
    tx.commit().await.unwrap();
    // Delivery enqueue follows replacement insertion and old-event closure.
    sqlx::raw_sql("CREATE FUNCTION reject_supersession_delivery() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'injected delivery failure'; END $$; CREATE TRIGGER reject_supersession_delivery BEFORE INSERT ON alertdeliveryoutbox FOR EACH ROW EXECUTE FUNCTION reject_supersession_delivery();")
        .execute(&f.pool).await.unwrap();
    let before = f.notifications.load(Ordering::SeqCst);
    assert!(
        f.alerts
            .process_pending_observation(Uuid::now_v7())
            .await
            .is_err()
    );
    f.resolved(old.id, false).await;
    assert_eq!(f.notifications.load(Ordering::SeqCst), before);
    let observed: DateTime<Utc> = sqlx::query_scalar(
        "SELECT lastobservedat FROM alertrulestates WHERE alertruleid=$1 AND resourceid=$2",
    )
    .bind(rule)
    .bind(resource)
    .fetch_one(&f.pool)
    .await
    .unwrap();
    assert_eq!(
        observed.timestamp_micros(),
        observation.observed_at.timestamp_micros(),
        "a failed transaction must also roll back the observation fence"
    );
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM alertevents WHERE resourceid=$1")
        .bind(resource)
        .fetch_one(&f.pool)
        .await
        .unwrap();
    assert_eq!(count, 1, "failed delivery must roll back the replacement");
    sqlx::raw_sql("DROP TRIGGER reject_supersession_delivery ON alertdeliveryoutbox; DROP FUNCTION reject_supersession_delivery(); UPDATE alertobservations SET availableat=CURRENT_TIMESTAMP;")
        .execute(&f.pool).await.unwrap();
    f.drain().await;
    let page = f
        .alerts
        .list_events(
            f.actor,
            true,
            &AlertEventFilter {
                resource_id: Some(resource),
                page_size: 50,
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(page.items.len(), 2);
    let new = page
        .items
        .iter()
        .find(|event| event.resolved_at.is_none())
        .unwrap();
    assert_superseded(&f, old.id, new.id).await;
    assert_eq!(f.notifications.load(Ordering::SeqCst), before + 1);
    let deliveries: i64 =
        sqlx::query_scalar("SELECT count(*) FROM alertdeliveryoutbox WHERE alerteventid=$1")
            .bind(new.id)
            .fetch_one(&f.pool)
            .await
            .unwrap();
    assert_eq!(deliveries, 1);
    assert!(f.alerts.process_event(&next).await.unwrap().is_none());
    assert!(
        f.alerts
            .process_event(&observation)
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(f.notifications.load(Ordering::SeqCst), before + 1);
    f.pool.close().await;
}
