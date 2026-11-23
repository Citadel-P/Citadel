use super::scoped_fixture::*;
use super::*;
use citadel_builds::BuildChange;
use citadel_server::realtime::build_change_callback;

async fn project(f: &Fixture, repo: Uuid) -> (Uuid, Vec<Uuid>) {
    let project = Uuid::now_v7();
    sqlx::query("INSERT INTO buildprojects(id,branch,createdbyactorid,gitrepositoryid,imagerepository,name,normalizedname,platformid)
                 VALUES($1,'main',$2,$3,'fixture',$4,$4,$5)")
        .bind(project).bind(SYSTEM_ACTOR_ID).bind(repo).bind(project.to_string()).bind(f.platform_id)
        .execute(&f.pool).await.unwrap();
    let runs = sqlx::query_scalar("INSERT INTO buildruns(id,branch,buildprojectid,contextpath,dockerfilepath,gitrepositoryid,gitrepositorynamesnapshot,imagerepository,platformsnapshot,projectnamesnapshot,registrysnapshot,status,timeoutseconds,trigger,triggeredbyactorid)
        SELECT gen_random_uuid(),'main',$1,'.','Dockerfile',$2,'bench','fixture','{}','fixture','null',CASE WHEN n=1 THEN 'Queued' ELSE 'Succeeded' END,60,'Manual',$3 FROM generate_series(1,10) n RETURNING id")
        .bind(project).bind(repo).bind(SYSTEM_ACTOR_ID).fetch_all(&f.pool).await.unwrap();
    (project, runs)
}

async fn watch(
    f: &Fixture,
    principal: &ActorPrincipal,
    address: std::net::SocketAddr,
    project: Uuid,
    run: Uuid,
) -> Socket {
    let mut socket = connect(f, principal, address).await;
    join(&mut socket, &format!("build-project:{project}"), 1).await;
    join(&mut socket, &format!("build-runs:{project}"), 10).await;
    join(&mut socket, &format!("build-run:{run}"), 1).await;
    socket
}

#[tokio::test]
#[ignore = "requires disposable CITADEL_PLATFORM_DATABASE_URL"]
async fn completion_updates_only_its_project_and_reconciles_retention() {
    let f = fixture().await;
    let repo = Uuid::now_v7();
    sqlx::query("INSERT INTO gitrepositories(id,createdbyactorid,defaultbranch,name,status,syncmode,url) VALUES($1,$2,'main','bench','Healthy','Manual','file:///unused')")
        .bind(repo).bind(SYSTEM_ACTOR_ID).execute(&f.pool).await.unwrap();
    let (a, runs_a) = project(&f, repo).await;
    let (b, runs_b) = project(&f, repo).await;
    let (hub, reads, address, cancellation, server) = serve(&f).await;
    let notify = build_change_callback(Some(hub.clone()));
    let restricted = super::super::lookup::subject(&f).await;
    grant(&f, restricted.actor_id.value(), ResourceType::Build, b, 0).await;
    let mut first = watch(&f, &f.administrator, address, a, runs_a[0]).await;
    let mut second = watch(&f, &restricted, address, b, runs_b[0]).await;
    assert!(
        !invoke(
            &mut second,
            "JoinGroup",
            &format!("build-run:{}", runs_a[0])
        )
        .await["error"]
            .is_null()
    );
    sqlx::query("UPDATE buildruns SET status='Succeeded',completedat=now() WHERE id=$1")
        .bind(runs_a[0])
        .execute(&f.pool)
        .await
        .unwrap();
    notify(BuildChange::ProjectRuns(a));
    barrier(&hub);
    let received = read_until_barrier(&mut first).await;
    assert_eq!(received.len(), 12);
    assert!(
        received
            .iter()
            .any(|e| e["arguments"][0]["id"] == json!(runs_a[0])
                && e["arguments"][0]["status"] == "Succeeded")
    );
    assert!(read_until_barrier(&mut second).await.is_empty());

    let affected = reads.take();
    assert_eq!(affected.len(), 3, "{affected:?}");
    eprintln!(
        "Build completion: {} group reads, {} wire rows, {} JSON bytes",
        affected.len(),
        received.len(),
        serde_json::to_vec(&received).unwrap().len()
    );
    // Keep an older run open while it disappears from the bounded run history.
    let old_group = format!("build-run:{}", runs_a[1]);
    join(&mut first, &old_group, 1).await;
    join(&mut first, "build-projects", 2).await;
    sqlx::query("DELETE FROM buildruns WHERE id=$1")
        .bind(runs_a[1])
        .execute(&f.pool)
        .await
        .unwrap();
    notify(BuildChange::ProjectRuns(a));
    barrier(&hub);
    let updates = read_until_barrier(&mut first).await;
    let projects: Vec<_> = updates
        .iter()
        .filter(|e| e["target"] == "BuildProjectInfoUpdated")
        .map(|e| &e["arguments"][0]["id"])
        .collect();
    assert_eq!(
        projects,
        vec![&json!(a), &json!(a)],
        "project detail and catalog patch"
    );
    let removals: Vec<_> = updates
        .iter()
        .filter(|e| e["arguments"][1] == "delete")
        .collect();
    assert_eq!(removals.len(), 2, "run list and open detail");
    assert!(
        removals
            .iter()
            .all(|e| e["arguments"][0]["id"] == json!(runs_a[1]))
    );
    assert!(removals.iter().any(|e| e["arguments"][2] == old_group));
    assert!(read_until_barrier(&mut second).await.is_empty());

    notify(BuildChange::Resync);
    barrier(&hub);
    assert_eq!(read_until_barrier(&mut second).await.len(), 12);
    assert!(
        read_until_barrier(&mut first)
            .await
            .iter()
            .any(|e| e["arguments"][0]["id"] == json!(b)),
        "full catalog recovery"
    );
    first.close(None).await.unwrap();
    second.close(None).await.unwrap();
    cancellation.cancel();
    server.await.unwrap();
    cleanup(f).await;
}

#[tokio::test]
#[ignore = "requires disposable CITADEL_PLATFORM_DATABASE_URL"]
async fn filtered_owner_changes_read_and_emit_only_the_matching_platform_or_project() {
    let f = fixture().await;
    let tag: String = sqlx::query_scalar("SELECT name FROM tags WHERE id=$1")
        .bind(f.tag_id)
        .fetch_one(&f.pool)
        .await
        .unwrap();
    let other_platform = Uuid::now_v7();
    sqlx::query("INSERT INTO platforms(id,address,connectortype,cpucount,imagecount,memtotal,name,networkcount,platformdescriptor,status,volumecount) SELECT $1,$1::text,connectortype,0,0,0,$1::text,0,platformdescriptor,status,0 FROM platforms WHERE id=$2")
        .bind(other_platform).bind(f.platform_id).execute(&f.pool).await.unwrap();
    let repo = Uuid::now_v7();
    sqlx::query("INSERT INTO gitrepositories(id,createdbyactorid,defaultbranch,name,status,syncmode,url) VALUES($1,$2,'main','filtered','Healthy','Manual','file:///unused')")
        .bind(repo).bind(SYSTEM_ACTOR_ID).execute(&f.pool).await.unwrap();
    let (matching_project, _) = project(&f, repo).await;
    let (other_project, _) = project(&f, repo).await;
    sqlx::query("INSERT INTO resourcetags(resourceid,resourcetype,tagid,createdbyactorid) VALUES($1,'Build',$2,$3)")
        .bind(matching_project).bind(f.tag_id).bind(SYSTEM_ACTOR_ID).execute(&f.pool).await.unwrap();
    let (hub, _, address, cancel, server) = serve(&f).await;
    let mut socket = connect(&f, &f.administrator, address).await;
    let count_queries = std::env::var_os("CITADEL_REQUIRE_QUERY_COUNTS").is_some();
    if count_queries {
        sqlx::query("CREATE EXTENSION IF NOT EXISTS pg_stat_statements")
            .execute(&f.pool)
            .await
            .unwrap();
    }
    for (group, kind, hit, miss, change) in [
        (
            "platforms",
            "Platform",
            f.platform_id,
            other_platform,
            "connectionChanged",
        ),
        (
            "build-projects",
            "Build",
            matching_project,
            other_project,
            "ownerChanged",
        ),
    ] {
        let subscription = Uuid::now_v7();
        let args = json!([{"subscriptionId":subscription,"group":group,"tags":[&tag]}]);
        assert!(invoke_args(&mut socket, "JoinGroup", args).await["error"].is_null());
        assert_eq!(
            receive(&mut socket).await["target"],
            "CollectionSnapshotStarted"
        );
        assert_eq!(receive(&mut socket).await["arguments"][0]["id"], json!(hit));
        let counters = || async {
            sqlx::query_as::<_,(i64,i64)>("SELECT coalesce(sum(calls),0)::bigint,coalesce(sum(rows),0)::bigint FROM pg_stat_statements WHERE dbid=(SELECT oid FROM pg_database WHERE datname=current_database()) AND query LIKE $1")
                .bind("%AND t.normalizedname=lower(requested.name)%").fetch_one(&f.pool).await.unwrap()
        };
        for (id, expected) in [(miss, 0), (hit, 1)] {
            let before = if count_queries {
                Some(counters().await)
            } else {
                None
            };
            hub.publish_resource_change(kind, id, change);
            barrier(&hub);
            let frames = read_until_barrier(&mut socket).await;
            assert_eq!(frames.len(), expected, "{kind} {id}: {frames:?}");
            if expected == 1 {
                assert_eq!(frames[0]["arguments"][0]["id"], json!(hit));
            }
            if let Some((calls, rows)) = before {
                let after = counters().await;
                assert_eq!(
                    (after.0 - calls, after.1 - rows),
                    (1, expected as i64),
                    "{kind} owner projection"
                );
            }
        }
        sqlx::query(
            "DELETE FROM resourcetags WHERE resourceid=$1 AND resourcetype=$2 AND tagid=$3",
        )
        .bind(hit)
        .bind(kind)
        .bind(f.tag_id)
        .execute(&f.pool)
        .await
        .unwrap();
        hub.publish_resource_change(kind, hit, change);
        barrier(&hub);
        let frames = read_until_barrier(&mut socket).await;
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0]["subscriptionId"], json!(subscription));
        if kind == "Platform" {
            assert_eq!(frames[0]["target"], "PlatformsDeleted");
            assert_eq!(frames[0]["arguments"][0], json!(hit));
        } else {
            assert_eq!(frames[0]["arguments"], json!([{"id":hit},"delete"]));
        }
        assert!(
            invoke(&mut socket, "LeaveGroup", &subscription.to_string()).await["error"].is_null()
        );
    }
    socket.close(None).await.unwrap();
    cancel.cancel();
    server.await.unwrap();
    cleanup(f).await;
}
