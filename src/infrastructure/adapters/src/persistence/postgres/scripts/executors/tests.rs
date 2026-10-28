use super::*;
use citadel_identity::IdentityStore;
#[tokio::test]
#[ignore = "requires disposable CITADEL_EXECUTION_DATABASE_URL"]
async fn remote_attempts_fence_tokens_deferral_shared_capacity_and_recovery() {
    let url = std::env::var("CITADEL_EXECUTION_DATABASE_URL").unwrap();
    citadel_database::MigrationRunner::migrate(&url)
        .await
        .unwrap();
    let pool = PgPool::connect(&url).await.unwrap();
    let store = PostgresScriptRepository::new(pool.clone());
    let identity =
        crate::persistence::postgres::identity::authentication::store::PostgresIdentityStore::new(
            pool.clone(),
        );
    let actor = ActorId::new(Uuid::now_v7());
    sqlx::query("INSERT INTO actors(id,isenabled,type) VALUES($1,true,'System')")
        .bind(actor.value())
        .execute(&pool)
        .await
        .unwrap();
    let agent = Uuid::now_v7();
    let mut input = citadel_scripts::ScriptConfiguration {
        name: Uuid::now_v7().to_string(),
        description: None,
        code: "console.log('fixture')".into(),
        duplicate_source_id: None,
        default_args_json: None,
        enabled: true,
        schedule_enabled: false,
        schedule_cron: None,
        schedule_time_zone: None,
        webhook: None,
        timeout_seconds: Some(30),
        alert_on_failure: false,
        run_as_actor_id: None,
        tag_ids: vec![],
        executor: citadel_scripts::ScriptExecutor::Agent { agent_id: agent },
    };
    input.validate(actor).unwrap();
    let script = store.create(actor, &input).await.unwrap();
    let run = store
        .enqueue(actor, script.id, "Manual", &serde_json::json!({}), None)
        .await
        .unwrap();
    assert!(
        store
            .claim_next(Utc::now() - chrono::Duration::minutes(10), false)
            .await
            .unwrap()
            .claim
            .is_none()
    );
    let first = store
        .claim_next(Utc::now() - chrono::Duration::minutes(10), true)
        .await
        .unwrap()
        .claim
        .unwrap();
    assert_eq!(first.run.id, run.id);
    let old = first.run.attempt_id.unwrap();
    assert!(
        identity
            .script_attempt_active(actor, run.id, old)
            .await
            .unwrap()
    );
    assert!(
        !identity
            .script_attempt_active(ActorId::new(Uuid::now_v7()), run.id, old)
            .await
            .unwrap()
    );
    store.defer(&first).await.unwrap();
    assert!(
        !identity
            .script_attempt_active(actor, run.id, old)
            .await
            .unwrap()
    );
    sqlx::query("UPDATE scriptruns SET nexteligibleat=now() WHERE id=$1")
        .bind(run.id)
        .execute(&pool)
        .await
        .unwrap();
    let claim = store
        .claim_next(Utc::now() - chrono::Duration::minutes(10), true)
        .await
        .unwrap()
        .claim
        .unwrap();
    assert_ne!(claim.run.attempt_id, Some(old));
    assert!(
        !identity
            .script_attempt_active(actor, run.id, old)
            .await
            .unwrap()
    );
    let target = Target {
        id: agent,
        kind: ResourceType::Platform,
        endpoint: None,
        available: true,
    };
    let host = Host {
        daemon: Uuid::now_v7().to_string(),
        authority: Uuid::now_v7(),
        generation: 1,
        slots: 4,
        heavy: 1,
    };
    assert!(reserve(&pool, &claim, &target, &host).await.unwrap());
    assert!(
        store.defer(&claim).await.is_err(),
        "uncertain dispatch must never requeue"
    );
    input.name = Uuid::now_v7().to_string();
    let second_action = store.create(actor, &input).await.unwrap();
    store
        .enqueue(
            actor,
            second_action.id,
            "Manual",
            &serde_json::json!({}),
            None,
        )
        .await
        .unwrap();
    let second = store
        .claim_next(Utc::now() - chrono::Duration::minutes(10), true)
        .await
        .unwrap()
        .claim
        .unwrap();
    assert!(
        !reserve(&pool, &second, &target, &host).await.unwrap(),
        "host heavy budget includes scripts"
    );
    let changed_host = Host {
        authority: Uuid::now_v7(),
        heavy: 8,
        ..host
    };
    assert!(
        !reserve(&pool, &second, &target, &changed_host)
            .await
            .unwrap(),
        "new authority cannot erase old occupancy"
    );
    store.cancel(script.id, run.id).await.unwrap();
    assert!(
        !identity
            .script_attempt_active(actor, run.id, claim.run.attempt_id.unwrap())
            .await
            .unwrap()
    );
    store
        .finish(
            &claim,
            &citadel_scripts::ScriptRunResult {
                status: citadel_scripts::ScriptRunStatus::Cancelled,
                exit_code: None,
                logs: String::new(),
                error: None,
            },
        )
        .await
        .unwrap();
    assert!(
        store
            .get_run(script.id, run.id)
            .await
            .unwrap()
            .cleanup_pending
    );
    assert_eq!(
        store.get(script.id).await.unwrap().current_run_id,
        Some(run.id)
    );
    let r = retained(&pool, run.id).await.unwrap().unwrap();
    confirm(
        &pool,
        &Reservation {
            authority: Uuid::now_v7(),
            ..Reservation {
                run: r.run,
                attempt: r.attempt,
                agent: r.agent,
                endpoint: r.endpoint.clone(),
                kind: r.kind,
                daemon: r.daemon.clone(),
                authority: r.authority,
                generation: r.generation,
            }
        },
    )
    .await
    .unwrap();
    assert!(
        store
            .get_run(script.id, run.id)
            .await
            .unwrap()
            .cleanup_pending,
        "wrong authority cannot release"
    );
    resolve(
        &pool,
        &r,
        actor,
        "Verified process stopped and host restarted",
    )
    .await
    .unwrap();
    assert!(resolve(&pool, &r, actor, "Duplicate").await.is_err());
    assert!(
        !store
            .get_run(script.id, run.id)
            .await
            .unwrap()
            .cleanup_pending
    );
    assert!(store.get(script.id).await.unwrap().current_run_id.is_none());
    assert!(
        reserve(&pool, &second, &target, &changed_host)
            .await
            .unwrap()
    );
    let r = retained(&pool, second.run.id).await.unwrap().unwrap();
    confirm(&pool, &r).await.unwrap();
    store
        .finish(
            &second,
            &citadel_scripts::ScriptRunResult {
                status: citadel_scripts::ScriptRunStatus::Succeeded,
                exit_code: Some(0),
                logs: String::new(),
                error: None,
            },
        )
        .await
        .unwrap();
    assert!(
        !store
            .get_run(second_action.id, second.run.id)
            .await
            .unwrap()
            .cleanup_pending
    );
    let audited: i64 = sqlx::query_scalar("SELECT count(*) FROM activityevents WHERE resourceid=$1 AND eventtype='ScriptRunCleanupResolved'").bind(script.id).fetch_one(&pool).await.unwrap();
    assert_eq!(audited, 1);
    input.name = Uuid::now_v7().to_string();
    let cancelled_action = store.create(actor, &input).await.unwrap();
    let run = store
        .enqueue(
            actor,
            cancelled_action.id,
            "Manual",
            &serde_json::json!({}),
            None,
        )
        .await
        .unwrap();
    let claim = store
        .claim_next(Utc::now() - chrono::Duration::minutes(10), true)
        .await
        .unwrap()
        .claim
        .unwrap();
    assert_eq!(claim.run.id, run.id);
    store.cancel(cancelled_action.id, run.id).await.unwrap();
    assert_eq!(
        store.defer(&claim).await.unwrap(),
        citadel_scripts::ScriptRunStatus::Cancelled
    );
    assert!(
        store
            .get(cancelled_action.id)
            .await
            .unwrap()
            .current_run_id
            .is_none(),
        "cancellation winning deferral must not leave queued work"
    );
}

#[tokio::test]
#[ignore = "requires disposable CITADEL_EXECUTION_DATABASE_URL"]
async fn automatic_discovers_only_authorized_agents_and_bounds_selection() {
    let url = std::env::var("CITADEL_EXECUTION_DATABASE_URL").unwrap();
    citadel_database::MigrationRunner::migrate(&url)
        .await
        .unwrap();
    let pool = PgPool::connect(&url).await.unwrap();
    let actor = ActorId::new(Uuid::now_v7());
    sqlx::query("INSERT INTO actors(id,isenabled,type) VALUES($1,true,'System')")
        .bind(actor.value())
        .execute(&pool)
        .await
        .unwrap();
    let seed = Uuid::now_v7();
    assert!(
        automatic_candidates(&pool, actor, false, seed)
            .await
            .unwrap()
            .is_empty()
    );
    let mut eligible = Vec::new();
    let mut platforms = Vec::new();
    for (connector, permission) in [
        ("Agent", 4),
        ("EdgeAgent", 4),
        ("Local", 4),
        ("Agent", 1),
        ("EdgeAgent", 0),
    ] {
        let id = Uuid::now_v7();
        sqlx::query("INSERT INTO platforms(id,address,connectortype,cpucount,imagecount,memtotal,name,networkcount,platformdescriptor,status,volumecount) VALUES($1,'https://agent.test/' || $1::text,$2,0,0,0,$1::text,0,'{\"$type\":\"Docker\"}','Online',0)")
            .bind(id).bind(connector).execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO resourceaccesses(id,actorid,permissionlevel,resourceid,resourcetype,specificpermissions) VALUES($1,$2,$3,$4,$5,0)")
            .bind(Uuid::now_v7()).bind(actor.value()).bind(permission).bind(id).bind(ResourceType::Platform as i32).execute(&pool).await.unwrap();
        if permission == 4 && connector != "Local" {
            eligible.push(id);
        }
        platforms.push(id);
    }
    let mut pools = Vec::new();
    for (enabled, archived, provider, permission) in [
        (true, false, "SelfManagedVm", 4),
        (false, false, "SelfManagedVm", 4),
        (true, true, "SelfManagedVm", 4),
        (true, false, "Docker", 4),
        (true, false, "SelfManagedVm", 1),
    ] {
        let id = Uuid::now_v7();
        sqlx::query("INSERT INTO buildagentpools(id,createdbyactorid,name,normalizedname,provider,providerspec,enabled,archivedat) VALUES($1,$2,$1::text,$1::text,$3,$4,$5,CASE WHEN $6 THEN now() END)")
            .bind(id).bind(actor.value()).bind(provider).bind(serde_json::json!({"$type":provider,"connectionMode":"EdgeAgent"})).bind(enabled).bind(archived).execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO resourceaccesses(id,actorid,permissionlevel,resourceid,resourcetype,specificpermissions) VALUES($1,$2,$3,$4,$5,0)")
            .bind(Uuid::now_v7()).bind(actor.value()).bind(permission).bind(id).bind(ResourceType::BuildAgentPool as i32).execute(&pool).await.unwrap();
        if enabled && !archived && provider == "SelfManagedVm" && permission == 4 {
            eligible.push(id);
        }
        pools.push(id);
    }
    let found = automatic_candidates(&pool, actor, false, seed)
        .await
        .unwrap();
    assert_eq!(found.len(), 3);
    assert!(eligible.iter().all(|id| found.contains(id)));
    sqlx::query("DELETE FROM resourceaccesses WHERE actorid=$1")
        .bind(actor.value())
        .execute(&pool)
        .await
        .unwrap();
    assert!(
        automatic_candidates(&pool, actor, false, seed)
            .await
            .unwrap()
            .is_empty(),
        "revocation changes discovery immediately"
    );
    let admin = automatic_candidates(&pool, actor, true, seed)
        .await
        .unwrap();
    assert!(eligible.iter().all(|id| admin.contains(id)));
    assert!(!admin.contains(&platforms[2]));
    assert!(!admin.contains(&pools[1]) && !admin.contains(&pools[2]) && !admin.contains(&pools[3]));
    for _ in 0..40 {
        let id = Uuid::now_v7();
        sqlx::query("INSERT INTO platforms(id,address,connectortype,cpucount,imagecount,memtotal,name,networkcount,platformdescriptor,status,volumecount) VALUES($1,'https://agent.test/' || $1::text,'Agent',0,0,0,$1::text,0,'{\"$type\":\"Docker\"}','Online',0)")
            .bind(id).execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO resourceaccesses(id,actorid,permissionlevel,resourceid,resourcetype,specificpermissions) VALUES($1,$2,4,$3,$4,0)")
            .bind(Uuid::now_v7()).bind(actor.value()).bind(id).bind(ResourceType::Platform as i32).execute(&pool).await.unwrap();
        platforms.push(id);
    }
    let first = automatic_candidates(&pool, actor, false, seed)
        .await
        .unwrap();
    let second = automatic_candidates(&pool, actor, false, Uuid::now_v7())
        .await
        .unwrap();
    assert_eq!(first.len(), 32);
    assert_eq!(second.len(), 32);
    assert_ne!(
        first, second,
        "new attempts rotate a large fleet's candidates"
    );
    sqlx::query("DELETE FROM resourceaccesses WHERE actorid=$1")
        .bind(actor.value())
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM buildagentpools WHERE id=ANY($1)")
        .bind(pools)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM platforms WHERE id=ANY($1)")
        .bind(platforms)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM actors WHERE id=$1")
        .bind(actor.value())
        .execute(&pool)
        .await
        .unwrap();
}
