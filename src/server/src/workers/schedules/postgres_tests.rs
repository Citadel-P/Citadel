//! Committed Stack edit → PostgreSQL hub → coordinator discovery and authorized catalog stream.
use crate::realtime_groups::schedule_stream as stream;
use crate::workers::{
    notifications::{DatabaseNotificationHub, ScopedTopic},
    schedules::{Dispatch, drive},
};
use chrono::{DateTime, Duration, Utc};
use citadel_adapters::persistence::postgres::{
    instance::{PostgresInstanceRepository, PostgresSchedules},
    notifications::PostgresNotifications,
};
use citadel_identity::{ActorPrincipal, AuthenticatedPrincipalType, SYSTEM_ACTOR_ID};
use citadel_instance::{InstanceService, SchedulePurpose as Purpose, ScheduleReader};
use futures_util::StreamExt;
use serde_json::json;
use sqlx::PgPool;
use std::{sync::Arc, time::Duration as Wait};
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

struct Catalog {
    rows: Arc<PostgresSchedules>,
    instance: Arc<InstanceService>,
    git: watch::Sender<Option<DateTime<Utc>>>,
}
impl Dispatch for Catalog {
    fn reader(&self) -> &dyn ScheduleReader {
        self.rows.as_ref()
    }
    fn notify(&self) {
        self.instance.notify_schedules();
    }
    fn set_deadline(&self, p: Purpose, at: DateTime<Utc>) {
        self.instance.set_schedule_deadline(p, at);
    }
    async fn deadline(&self, p: Purpose) -> Result<Option<DateTime<Utc>>, String> {
        if p != Purpose::Git {
            return Ok(None);
        }
        let due = self.rows.batch_deadline(p).await.unwrap();
        self.git.send_replace(due);
        Ok(due)
    }
    async fn occurrence(
        &self,
        _: Purpose,
        _: Uuid,
        _: DateTime<Utc>,
        _: &CancellationToken,
    ) -> Result<bool, String> {
        panic!("unexpected calendar admission")
    }
    async fn batch(
        &self,
        _: Purpose,
        _: Option<uuid::Uuid>,
        _: &CancellationToken,
    ) -> Result<(), String> {
        panic!("unexpected batch before startup delay")
    }
}

#[tokio::test]
#[ignore = "requires an isolated CITADEL_EXECUTION_DATABASE_URL"]
async fn committed_stack_edits_wake_git_and_the_authorized_schedule_catalog() {
    let url = std::env::var("CITADEL_EXECUTION_DATABASE_URL").unwrap();
    citadel_database::MigrationRunner::migrate(&url)
        .await
        .unwrap();
    let pool = PgPool::connect(&url).await.unwrap();
    let (first, second, stack, release, historical, platform, offline) = (
        Uuid::now_v7(),
        Uuid::now_v7(),
        Uuid::now_v7(),
        Uuid::now_v7(),
        Uuid::now_v7(),
        Uuid::now_v7(),
        Uuid::now_v7(),
    );
    for repo in [first, second] {
        sqlx::query("INSERT INTO gitrepositories(id,name,url,createdbyactorid,defaultbranch,syncmode,syncintervalminutes,status) VALUES($1,$1::text,'https://example.invalid/git',$2,'main','PullInterval',240,'Healthy')")
        .bind(repo).bind(SYSTEM_ACTOR_ID).execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO gitrepositoryrefs(id,gitrepositoryid,branch,status,lastsyncedat) VALUES($1,$2,'main','Healthy',now())")
        .bind(Uuid::now_v7()).bind(repo).execute(&pool).await.unwrap();
    }
    for (id, status) in [(platform, "Online"), (offline, "Offline")] {
        sqlx::query(
            r#"INSERT INTO platforms(id,address,connectortype,cpucount,imagecount,memtotal,name,networkcount,platformdescriptor,status,volumecount)
               VALUES($1,$1::text,'Local',0,0,0,$1::text,0,'{"$type":"Docker"}',$2,0)"#,
        )
        .bind(id)
        .bind(status)
        .execute(&pool)
        .await
        .unwrap();
    }
    sqlx::query("INSERT INTO stacks(id,name,currentstackreleaseid,createdbyactorid,stacksource,driftpolicy,stackupdatestate) VALUES($1,$1::text,$2,$3,'Git','{}','{}')").bind(stack).bind(release).bind(SYSTEM_ACTOR_ID).execute(&pool).await.unwrap();
    let spec = |repo: Uuid, branch: &str, pin: &str| json!({"GitRepoId":repo,"Branch":branch,"CommitSha":pin,"UpdateBehavior":"Notify"});
    for id in [release, historical] {
        sqlx::query("INSERT INTO stackreleases(id,stackid,platformid,createdbyactorid,spec,status,version) VALUES($1,$2,$3,$4,$5,'Healthy','1')")
        .bind(id).bind(stack).bind(platform).bind(SYSTEM_ACTOR_ID).bind(spec(first,"main","")).execute(&pool).await.unwrap();
    }
    let rows = Arc::new(PostgresSchedules(pool.clone()));
    let instance = Arc::new(
        InstanceService::new(
            Arc::new(PostgresInstanceRepository::new(pool.clone())),
            Arc::new(citadel_identity::SystemClock),
        )
        .with_schedules(rows.clone()),
    );
    let mut admin = ActorPrincipal {
        subject_id: Uuid::now_v7(),
        actor_id: citadel_primitives::ActorId::new(Uuid::now_v7()),
        name: "admin".into(),
        principal_type: AuthenticatedPrincipalType::User,
        credential_id: None,
        roles: vec![],
    };
    let stop = CancellationToken::new();
    assert!(stream(&instance, &admin, &stop).is_err());
    admin.roles.push("Admin".into());
    for (page, purpose) in [(1, Purpose::Install), (2, Purpose::Release)] {
        let rows = instance
            .schedules(&admin, Some("instance"), "core", page, 1, false)
            .await
            .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].purpose, purpose);
    }
    for (resource, page, size) in [("unknown", 1, 25), ("stack", 0, 25), ("stack", 1, 201)] {
        assert!(matches!(
            instance
                .schedules(&admin, Some(resource), "", page, size, false)
                .await,
            Err(citadel_instance::InstanceError::Validation(_))
        ));
    }
    let hub = DatabaseNotificationHub::new();
    let scoped = hub.scoped(ScopedTopic::Schedules);
    let mut audit = hub.scoped(ScopedTopic::Schedules);
    let listener = PostgresNotifications::connect(&pool, "citadel_schedules")
        .await
        .unwrap();
    let hub_task = tokio::spawn(hub.run(Box::new(listener), stop.clone()));
    let (g, mut deadlines) = watch::channel(None);
    let (_git, git) = watch::channel((0, uuid::Uuid::nil()));
    let (_license, license) = watch::channel(());
    let coordinator = tokio::spawn(drive(
        Catalog {
            rows: rows.clone(),
            instance: instance.clone(),
            git: g,
        },
        scoped,
        git,
        license,
        stop.clone(),
    ));
    tokio::time::timeout(
        Wait::from_secs(3),
        deadlines.wait_for(|d| d.is_some_and(|t| t > Utc::now() + Duration::hours(3))),
    )
    .await
    .unwrap()
    .unwrap();
    let mut updates = stream(&instance, &admin, &stop).unwrap();
    updates.next().await.unwrap().unwrap();
    const EDIT: &str = "UPDATE stackreleases SET spec=$2 WHERE id=$1";
    let mut rollback = pool.begin().await.unwrap();
    sqlx::query(EDIT)
        .bind(release)
        .bind(spec(first, "feature", ""))
        .execute(&mut *rollback)
        .await
        .unwrap();
    rollback.rollback().await.unwrap();
    sqlx::query(EDIT)
        .bind(historical)
        .bind(spec(second, "history", ""))
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE stackreleases SET source='{}' WHERE id=$1")
        .bind(release)
        .execute(&pool)
        .await
        .unwrap();

    for (repo, pin, due_now) in [
        (first, "", true),
        (second, "", true),
        (second, "commit", false),
        (second, "", true),
    ] {
        deadlines.borrow_and_update();
        sqlx::query(EDIT)
            .bind(release)
            .bind(spec(repo, "feature", pin))
            .execute(&pool)
            .await
            .unwrap();
        let event = tokio::time::timeout(Wait::from_secs(3), audit.recv())
            .await
            .unwrap()
            .unwrap();
        assert!(
            matches!(event,citadel_runtime::notifications::RuntimeNotification::Schedule(ref s) if s=="Git")
        );
        tokio::time::timeout(Wait::from_secs(3), deadlines.changed())
            .await
            .unwrap()
            .unwrap();
        tokio::time::timeout(
            Wait::from_secs(3),
            deadlines.wait_for(|d| {
                d.is_some_and(|t| (t <= Utc::now() + Duration::seconds(5)) == due_now)
            }),
        )
        .await
        .unwrap()
        .unwrap();
        let event = tokio::time::timeout(Wait::from_secs(3), updates.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert_eq!(event.target, "InstanceSchedulesChanged");
        let catalog = instance
            .schedules(&admin, Some("git-repository"), "", 1, 25, false)
            .await
            .unwrap();
        assert_eq!(catalog.len(), 2);
        for row in catalog {
            assert_eq!(
                row.anchor.unwrap() <= Utc::now() + Duration::seconds(5),
                due_now && row.owner == repo
            );
        }
    }
    sqlx::query(
        "UPDATE stackreleases SET spec=spec || '{\"UpdateBehavior\":\"None\"}' WHERE id=$1",
    )
    .bind(release)
    .execute(&pool)
    .await
    .unwrap();
    // Also fences the preceding rollback, historical edit and observation write:
    // an extra Git wake would still be ahead of this distinct message.
    assert!(
        matches!(tokio::time::timeout(Wait::from_secs(3),audit.recv()).await.unwrap().unwrap(),citadel_runtime::notifications::RuntimeNotification::Schedule(s) if s=="Catalog")
    );
    tokio::time::timeout(Wait::from_secs(3), updates.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert!(
        !instance
            .schedules(&admin, Some("stack"), "", 1, 25, false)
            .await
            .unwrap()
            .iter()
            .find(|row| row.purpose == Purpose::StackGit)
            .unwrap()
            .enabled
    );
    sqlx::query("UPDATE stackreleases SET platformid=$2 WHERE id=$1")
        .bind(release)
        .bind(offline)
        .execute(&pool)
        .await
        .unwrap();
    assert!(
        matches!(tokio::time::timeout(Wait::from_secs(3),audit.recv()).await.unwrap().unwrap(),citadel_runtime::notifications::RuntimeNotification::Schedule(s) if s=="Catalog")
    );
    tokio::time::timeout(Wait::from_secs(3), updates.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    let catalog = instance
        .schedules(&admin, Some("stack"), "", 1, 25, false)
        .await
        .unwrap();
    let row = catalog
        .iter()
        .find(|row| row.purpose == Purpose::StackGit)
        .unwrap();
    assert_eq!(row.platform, Some(offline));
    assert_eq!(row.blocker.as_deref(), Some("Platform is offline."));
    assert_eq!(row.resource_status.as_deref(), Some("Healthy"));

    sqlx::query("UPDATE gitrepositories SET status='Degraded' WHERE id=$1")
        .bind(first)
        .execute(&pool)
        .await
        .unwrap();
    assert!(
        matches!(tokio::time::timeout(Wait::from_secs(3),audit.recv()).await.unwrap().unwrap(),citadel_runtime::notifications::RuntimeNotification::Schedule(s) if s=="Catalog")
    );
    let event = tokio::time::timeout(Wait::from_secs(3), updates.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(event.target, "InstanceSchedulesChanged");
    let git = instance
        .schedules(&admin, Some("git-repository"), "", 1, 25, false)
        .await
        .unwrap();
    assert_eq!(
        git.iter()
            .find(|r| r.owner == first)
            .unwrap()
            .resource_status
            .as_deref(),
        Some("Degraded")
    );
    stop.cancel();
    coordinator.await.unwrap().unwrap();
    hub_task.await.unwrap().unwrap();
}
