use super::*;

#[test]
fn targeted_generation_and_negative_cache_are_bounded() {
    let cache = AuthorizationCache {
        gate: Arc::new(RwLock::new(())),
        state: Mutex::new(State::default()),
    };
    let alice = Uuid::now_v7();
    let bob = Uuid::now_v7();
    let alice_watch = cache.subscribe(alice);
    let bob_watch = cache.subscribe(bob);
    let generation = *alice_watch.borrow();
    cache.state.lock().unwrap().put_resource(
        (alice, 1, Uuid::now_v7()),
        generation,
        None,
        Instant::now() + SAFETY_TTL,
    );
    cache.invalidate([alice]);
    assert!(alice_watch.has_changed().unwrap());
    assert!(!bob_watch.has_changed().unwrap());
    assert_ne!(*alice_watch.borrow(), generation);
    let mut state = cache.state.lock().unwrap();
    assert!(state.resources.values().next().unwrap().grant.is_none());
    for _ in 0..ACTOR_CAPACITY + 1 {
        state.actor(Uuid::now_v7());
    }
    for _ in 0..RESOURCE_CAPACITY + 1 {
        state.put_resource(
            (alice, 1, Uuid::now_v7()),
            generation,
            None,
            Instant::now() + SAFETY_TTL,
        );
    }
    assert_eq!(state.actors.len(), ACTOR_CAPACITY);
    assert_eq!(state.resources.len(), RESOURCE_CAPACITY);
    assert!(alice_watch.has_changed().is_err());
    assert_eq!(state.actor_order.len(), ACTOR_CAPACITY);
    assert_eq!(state.resource_order.len(), RESOURCE_CAPACITY);
    let replacement = *state.actor(alice).generation.borrow();
    assert_ne!(replacement, generation);
    let key = (alice, 1, Uuid::now_v7());
    state.put_resource(key, generation, None, Instant::now() + SAFETY_TTL);
    assert!(
        state.resource(key, replacement).is_none(),
        "old negative entry cannot match a readmitted actor"
    );
}

#[test]
fn resource_entry_cannot_outlive_the_scope_it_used() {
    let mut state = State::default();
    let key = (Uuid::now_v7(), 1, Uuid::now_v7());
    let generation = Uuid::now_v7();
    state.put_resource(key, generation, None, Instant::now());
    assert!(state.resource(key, generation).is_none());
}

#[tokio::test]
async fn authorization_read_fence_blocks_acl_mutation_until_claim_finishes() {
    let cache = Arc::new(AuthorizationCache {
        gate: Arc::new(RwLock::new(())),
        state: Mutex::new(State::default()),
    });
    let fence = cache.read_fence().await;
    let gate = cache.gate.clone();
    let mut mutation = tokio::spawn(async move { gate.write_owned().await });
    assert!(
        tokio::time::timeout(Duration::from_millis(20), &mut mutation)
            .await
            .is_err()
    );
    drop(fence);
    tokio::time::timeout(Duration::from_secs(1), mutation)
        .await
        .expect("ACL mutation should proceed after claim commit")
        .unwrap();
}

#[tokio::test]
#[ignore = "requires CITADEL_IDENTITY_DATABASE_URL"]
async fn cancelled_commit_retains_gate_and_publishes_invalidation() {
    use citadel_primitives::ActorId;
    let url = std::env::var("CITADEL_IDENTITY_DATABASE_URL").unwrap();
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(4)
        .connect(&url)
        .await
        .unwrap();
    let cache = AuthorizationCache::attach(&pool);
    let actor = Uuid::now_v7();
    let mut changes = cache.subscribe(actor);
    let mut blocker = pool.acquire().await.unwrap();
    let key = (actor.as_u128() & i64::MAX as u128) as i64;
    sqlx::query("SELECT pg_advisory_lock($1)")
        .bind(key)
        .execute(&mut *blocker)
        .await
        .unwrap();
    let (started, ready) = tokio::sync::oneshot::channel();
    let task_pool = pool.clone();
    let caller = tokio::spawn(async move {
        let mutation = Mutation::enter(&task_pool).await;
        let mut tx = task_pool.begin().await.unwrap();
        let sql = format!(
            "CREATE TEMP TABLE authorization_commit_probe (id int) ON COMMIT DROP; CREATE FUNCTION pg_temp.authorization_commit_pause() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN PERFORM pg_advisory_xact_lock({key}::bigint); RETURN NEW; END $$; CREATE CONSTRAINT TRIGGER commit_pause AFTER INSERT ON authorization_commit_probe DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION pg_temp.authorization_commit_pause(); INSERT INTO authorization_commit_probe VALUES (1)"
        );
        sqlx::raw_sql(sqlx::AssertSqlSafe(sql.as_str()))
            .execute(&mut *tx)
            .await
            .unwrap();
        started.send(()).unwrap();
        mutation
            .commit(tx, Impact::Actors(vec![actor]))
            .await
            .unwrap();
    });
    ready.await.unwrap();
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let waiting: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_locks WHERE locktype='advisory' AND NOT granted AND classid=$1::bigint::oid AND objid=$2::bigint::oid)")
                .bind(key >> 32).bind(key & 0xffff_ffff).fetch_one(&pool).await.unwrap();
            if waiting { break; }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }).await.unwrap();
    caller.abort();
    assert!(caller.await.unwrap_err().is_cancelled());
    assert!(
        cache.gate.try_read().is_err(),
        "detached COMMIT must retain its write guard"
    );
    assert!(
        !changes.has_changed().unwrap(),
        "no invalidation before commit"
    );
    sqlx::query("SELECT pg_advisory_unlock($1)")
        .bind(key)
        .execute(&mut *blocker)
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(3), changes.changed())
        .await
        .unwrap()
        .unwrap();
    assert!(
        !cache
            .snapshot(&pool, ActorId::new(actor))
            .await
            .unwrap()
            .enabled
    );
    drop(blocker);
    pool.close().await;
}
