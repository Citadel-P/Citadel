use super::*;

#[tokio::test]
#[ignore = "requires an isolated CITADEL_EXECUTION_DATABASE_URL"]
async fn admission_rechecks_repository_edits_committed_after_selection() {
    let url = std::env::var("CITADEL_EXECUTION_DATABASE_URL").unwrap();
    MigrationRunner::migrate(&url).await.unwrap();
    let writer = PgPoolOptions::new()
        .max_connections(1)
        .connect(&url)
        .await
        .unwrap();
    let reader = PgPoolOptions::new()
        .max_connections(1)
        .connect(&url)
        .await
        .unwrap();
    let id = Uuid::now_v7();
    insert_repository(
        &writer,
        citadel_identity::SYSTEM_ACTOR_ID,
        id,
        "https://example.invalid/git",
        "claim",
    )
    .await;
    sqlx::query("UPDATE gitrepositories SET syncmode='PullInterval',syncintervalminutes=1,controlstate='Idle' WHERE id=$1").bind(id).execute(&writer).await.unwrap();
    sqlx::query("UPDATE gitrepositoryrefs SET status='Healthy',lastsyncedat=now()-interval '10 minutes' WHERE gitrepositoryid=$1").bind(id).execute(&writer).await.unwrap();
    sqlx::query("INSERT INTO gitrepositoryrefs(id,gitrepositoryid,branch,status,lastsyncedat) VALUES($1,$2,'next','Healthy',now())").bind(Uuid::now_v7()).bind(id).execute(&writer).await.unwrap();

    // Pause the real candidate query after its statement snapshot exists, before LockRows.
    // Only the dedicated reader connection sees this temporary view.
    sqlx::raw_sql(
        "CREATE TEMP VIEW git_schedule_branches AS
        SELECT b.* FROM public.git_schedule_branches b
        CROSS JOIN LATERAL pg_advisory_xact_lock(hashtextextended(b.id::text,0)) gate",
    )
    .execute(&reader)
    .await
    .unwrap();
    let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&reader)
        .await
        .unwrap();
    let mut edit = writer.begin().await.unwrap();
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1::text,0))")
        .bind(id.to_string())
        .execute(&mut *edit)
        .await
        .unwrap();
    let service = PostgresGitRepositoryExecutionPersistence::new(reader);
    let admission = tokio::spawn(async move { service.enqueue_due(100).await });
    tokio::time::timeout(Duration::from_secs(5), async {
        while !sqlx::query_scalar::<_, bool>("SELECT pg_backend_pid()=ANY(pg_blocking_pids($1))")
            .bind(pid)
            .fetch_one(&mut *edit)
            .await
            .unwrap()
        {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    sqlx::query(
        "UPDATE gitrepositories SET defaultbranch='next',syncintervalminutes=60 WHERE id=$1",
    )
    .bind(id)
    .execute(&mut *edit)
    .await
    .unwrap();
    edit.commit().await.unwrap();
    assert_eq!(admission.await.unwrap().unwrap(), 0);
    let state: String = sqlx::query_scalar("SELECT controlstate FROM gitrepositories WHERE id=$1")
        .bind(id)
        .fetch_one(&writer)
        .await
        .unwrap();
    assert_eq!(state, "Idle");
    // The edited schedule can admit its new branch when that branch really becomes due.
    sqlx::query("UPDATE gitrepositoryrefs SET lastsyncedat=now()-interval '2 hours' WHERE gitrepositoryid=$1 AND branch='next'")
        .bind(id).execute(&writer).await.unwrap();
    let current = PostgresGitRepositoryExecutionPersistence::new(writer.clone());
    assert_eq!(current.enqueue_due(100).await.unwrap(), 1);
    let pending: Vec<String> = sqlx::query_scalar(
        "SELECT branch FROM gitrepositoryrefs WHERE gitrepositoryid=$1 AND status='Pending'",
    )
    .bind(id)
    .fetch_all(&writer)
    .await
    .unwrap();
    assert_eq!(pending, ["next"]);
    sqlx::query("DELETE FROM gitrepositories WHERE id=$1")
        .bind(id)
        .execute(&writer)
        .await
        .unwrap();
}
