use super::*;

#[tokio::test]
#[ignore = "requires disposable CITADEL_EXECUTION_DATABASE_URL"]
async fn tagged_alerts_require_both_grants_before_pagination() {
    let url = std::env::var("CITADEL_EXECUTION_DATABASE_URL").unwrap();
    MigrationRunner::migrate(&url).await.unwrap();
    let pool = PgPoolOptions::new().connect(&url).await.unwrap();
    let actor = ActorId::new(Uuid::now_v7());
    sqlx::query("INSERT INTO actors(id,isenabled,type) VALUES($1,true,'User')")
        .bind(actor.value())
        .execute(&pool)
        .await
        .unwrap();
    let tag = Uuid::now_v7();
    let name = format!("tag-{tag}");
    sqlx::query("INSERT INTO tags(id,name,normalizedname,color,createdbyactorid) VALUES($1,$2,$2,'#abcdef',$3)")
        .bind(tag).bind(&name).bind(actor.value()).execute(&pool).await.unwrap();
    let rule = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO alertrules(id,name,type,severity,limitedto,quiethours,createdbyactorid)
        VALUES($1,$1::uuid::text,'PlatformCpuHigh','Warning','[]','[]',$2)",
    )
    .bind(rule)
    .bind(actor.value())
    .execute(&pool)
    .await
    .unwrap();
    let mut expected = Vec::new();
    for (tagged, may_read_platform, may_read_event) in [
        (true, true, true),
        (true, true, true),
        (true, false, true),
        (true, true, false),
        (false, true, true),
    ] {
        let platform = Uuid::now_v7();
        let event = Uuid::now_v7();
        sqlx::query("INSERT INTO platforms(id,address,connectortype,cpucount,imagecount,memtotal,name,networkcount,platformdescriptor,status,volumecount)
            VALUES($1,$1::uuid::text,'Local',1,1,1,$1::uuid::text,0,'{}','Online',0)")
            .bind(platform).execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO alertevents(id,alertruleid,type,severity,info,resourceid,resourcename,resourcetype,deduplicationkey)
            VALUES($1,$2,'PlatformCpuHigh','Warning','{}',$3,'host','Platform',$1::uuid::text)")
            .bind(event).bind(rule).bind(platform).execute(&pool).await.unwrap();
        if tagged {
            sqlx::query("INSERT INTO resourcetags(resourcetype,resourceid,tagid,createdbyactorid) VALUES('Platform',$1,$2,$3)")
                .bind(platform).bind(tag).bind(actor.value()).execute(&pool).await.unwrap();
        }
        for (id, kind, grant) in [
            (
                platform,
                citadel_primitives::ResourceType::Platform,
                may_read_platform,
            ),
            (
                event,
                citadel_primitives::ResourceType::Alert,
                may_read_event,
            ),
        ] {
            if grant {
                sqlx::query("INSERT INTO resourceaccesses(id,actorid,resourcetype,resourceid,permissionlevel,specificpermissions) VALUES($1,$2,$3,$4,1,0)")
                    .bind(Uuid::now_v7()).bind(actor.value()).bind(kind as i32).bind(id).execute(&pool).await.unwrap();
            }
        }
        if tagged && may_read_platform && may_read_event {
            expected.push(event);
        }
    }
    // A sync failure stores the Git repository's identity under the Webhook event kind.
    // A generic listener failure with the same UUID has no typed owner.
    let git = Uuid::now_v7();
    sqlx::query("INSERT INTO gitrepositories(id,name,url,createdbyactorid,defaultbranch,status) VALUES($1,$1::uuid::text,'https://example.invalid/repo',$2,'main','Unknown')")
        .bind(git).bind(actor.value()).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO resourcetags(resourcetype,resourceid,tagid,createdbyactorid) VALUES('GitRepository',$1,$2,$3)")
        .bind(git).bind(tag).bind(actor.value()).execute(&pool).await.unwrap();
    for kind in ["WebhookGitRepoSyncFailed", "WebhookAuthenticationFailed"] {
        let id = Uuid::now_v7();
        sqlx::query("INSERT INTO alertevents(id,alertruleid,type,severity,info,resourceid,resourcename,resourcetype,deduplicationkey)
            VALUES($1,$2,$3,'Warning','{}',$4,'webhook','Webhook',$1::uuid::text)")
            .bind(id).bind(rule).bind(kind).bind(git).execute(&pool).await.unwrap();
    }
    let repo = PostgresAlertRepository::new(pool);
    let webhooks = repo
        .list_events(
            actor,
            true,
            &AlertEventFilter {
                tags: vec![name.clone()],
                resource_type: Some("Webhook".into()),
                page_size: 5,
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(webhooks.total_count, 1);
    assert_eq!(webhooks.items[0].alert_type, "WebhookGitRepoSyncFailed");
    for (page, expected_id) in (1..).zip(expected.into_iter().rev()) {
        let filter = AlertEventFilter {
            tags: vec![name.to_uppercase()],
            page,
            page_size: 1,
            unresolved_only: true,
            ..Default::default()
        };
        let rows = repo.list_events(actor, false, &filter).await.unwrap();
        assert_eq!(rows.total_count, 2);
        assert_eq!(
            rows.items.iter().map(|e| e.id).collect::<Vec<_>>(),
            [expected_id]
        );
    }
}
