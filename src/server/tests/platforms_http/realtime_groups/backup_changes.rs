use super::scoped_fixture::*;
use super::*;

async fn statement_totals(pool: &sqlx::PgPool) -> (i64, i64) {
    sqlx::query_as("SELECT COALESCE(sum(calls),0)::bigint, COALESCE(sum(rows),0)::bigint FROM pg_stat_statements WHERE dbid=(SELECT oid FROM pg_database WHERE datname=current_database()) AND query NOT ILIKE '%pg_stat_statements%'")
        .fetch_one(pool).await.unwrap()
}

#[tokio::test]
#[ignore = "requires disposable CITADEL_PLATFORM_DATABASE_URL"]
async fn committed_policy_change_refreshes_its_run_and_one_authorized_catalog_row() {
    let f = fixture().await;
    let secret = Uuid::now_v7();
    let repository = Uuid::now_v7();
    sqlx::query("INSERT INTO secretdefinitions(id,name,providertype) VALUES($1,$1::text,'InternalEncrypted')")
        .bind(secret).execute(&f.pool).await.unwrap();
    sqlx::query("INSERT INTO backuprepositories(id,name,normalizedname,passwordsecretid,spec,status,type,createdbyactorid) VALUES($1,$1::text,$1::text,$2,'{}','Ready','FileSystem',$3)")
        .bind(repository).bind(secret).bind(SYSTEM_ACTOR_ID).execute(&f.pool).await.unwrap();
    let [a, b, run_a, run_b] = std::array::from_fn(|_| Uuid::now_v7());
    for (policy, run) in [(a, run_a), (b, run_b)] {
        sqlx::query(r#"INSERT INTO backuppolicies(id,name,normalizedname,backuprepositoryid,createdbyactorid,runasactorid,source) VALUES($1,$1::text,$1::text,$2,$3,$3,'{"$type":"CitadelSystem"}')"#)
            .bind(policy).bind(repository).bind(SYSTEM_ACTOR_ID).execute(&f.pool).await.unwrap();
        sqlx::query(r#"INSERT INTO backupruns(id,backuppolicyid,backuprepositoryid,policynamesnapshot,sourcesnapshot,repositorytypesnapshot,trigger,status,snapshotavailability,triggeredbyactorid) VALUES($1,$2,$3,'fixture','{"$type":"CitadelSystem"}','FileSystem','Manual','Queued','Pending',$4)"#)
            .bind(run).bind(policy).bind(repository).bind(SYSTEM_ACTOR_ID).execute(&f.pool).await.unwrap();
    }
    let (hub, reads, address, cancellation, server) = serve(&f).await;
    let restricted = super::super::lookup::subject(&f).await;
    grant(
        &f,
        restricted.actor_id.value(),
        ResourceType::BackupPolicy,
        b,
        0,
    )
    .await;
    let mut first = connect(&f, &f.administrator, address).await;
    let mut second = connect(&f, &restricted, address).await;
    for (socket, policy, run) in [(&mut first, a, run_a), (&mut second, b, run_b)] {
        join(socket, &format!("backup-policy:{policy}"), 1).await;
        join(socket, &format!("backup-runs:{policy}"), 1).await;
        join(socket, &format!("backup-run:{run}"), 1).await;
    }
    join(&mut first, "backup-policies", 2).await;
    sqlx::query("UPDATE backupruns SET status='Succeeded',completedat=now() WHERE id=$1")
        .bind(run_a)
        .execute(&f.pool)
        .await
        .unwrap();
    let notify = citadel_server::realtime::backup_change_callback(Some(hub.clone()));
    let before = if std::env::var_os("CITADEL_REQUIRE_QUERY_COUNTS").is_some() {
        sqlx::query("CREATE EXTENSION IF NOT EXISTS pg_stat_statements")
            .execute(&f.pool)
            .await
            .unwrap();
        Some(statement_totals(&f.pool).await)
    } else {
        None
    };
    notify(citadel_backups::BackupChange::Run {
        policy: a,
        run: run_a,
        retention: true,
    });
    barrier(&hub);
    let rows = read_until_barrier(&mut first).await;
    assert_eq!(rows.len(), 4, "{rows:?}");
    assert!(rows.iter().all(|e| e["arguments"][0]["id"]==json!(a) || e["arguments"][0]["id"]==json!(run_a)));
    assert!(read_until_barrier(&mut second).await.is_empty());
    let affected = reads.take();
    if let Some((calls, rows)) = before {
        let after = statement_totals(&f.pool).await;
        eprintln!(
            "Backup completion SQL: {} calls, {} rows",
            after.0 - calls,
            after.1 - rows
        );
    }
    assert_eq!(affected.len(), 4, "{affected:?}");
    assert!(
        affected
            .iter()
            .all(|name| !name.contains(&b.to_string()) && !name.contains(&run_b.to_string()))
    );
    eprintln!(
        "Backup completion: {} group reads, {} wire rows, {} JSON bytes",
        affected.len(),
        rows.len(),
        serde_json::to_vec(&rows).unwrap().len()
    );
    sqlx::query("UPDATE backuppolicies SET archivedat=now() WHERE id=$1")
        .bind(a)
        .execute(&f.pool)
        .await
        .unwrap();
    notify(citadel_backups::BackupChange::Policy(a));
    barrier(&hub);
    assert!(
        read_until_barrier(&mut first)
            .await
            .iter()
            .any(|e| e["arguments"][0]["id"] == json!(a) && e["arguments"][1] == "delete")
    );
    assert!(read_until_barrier(&mut second).await.is_empty());
    first.close(None).await.unwrap();
    second.close(None).await.unwrap();
    cancellation.cancel();
    server.await.unwrap();
    cleanup(f).await;
}
