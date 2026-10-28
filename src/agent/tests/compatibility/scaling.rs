//! Real Core executor, PostgreSQL admission, Direct/Edge transfer and Docker work.
use super::*;
use citadel_adapters::{
    external::builds::executor::AgentDockerBuildExecutor,
    persistence::postgres::builds::PostgresBuildRepository,
};
use citadel_builds::*;
use citadel_contracts::citadel::scripts::v1::*;
use futures_util::future::BoxFuture;
use std::sync::atomic::{AtomicUsize, Ordering};

struct NoCredentials;
impl BuildSecretResolver for NoCredentials {
    fn resolve(
        &self,
        _: Uuid,
        _: Uuid,
    ) -> BoxFuture<'_, Result<zeroize::Zeroizing<String>, BuildError>> {
        Box::pin(async { panic!("fixture has no secrets") })
    }
}
impl BuildRegistryCredentialResolver for NoCredentials {
    fn resolve(
        &self,
        _: Uuid,
    ) -> BoxFuture<'_, Result<Option<BuildRegistryCredentials>, BuildError>> {
        Box::pin(async { panic!("fixture does not push") })
    }
}
#[derive(Default)]
struct Logs(AtomicUsize);
impl BuildLogSink for Logs {
    fn append<'a>(&'a self, _: &'a str, message: &'a str) -> BoxFuture<'a, Result<(), BuildError>> {
        Box::pin(async move {
            self.0.fetch_add(message.len(), Ordering::SeqCst);
            Ok(())
        })
    }
}
fn git(path: &std::path::Path, args: &[&str]) {
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(path)
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[tokio::test]
#[ignore = "requires test-agent-compatibility.sh with three isolated Docker daemons and current Agent image"]
async fn six_real_remote_builds_overlap_with_two_preparation_permits() {
    let hosts =
        std::env::var("CITADEL_SCALING_DOCKER_HOSTS").expect("three disposable harness daemons");
    let hosts = hosts.split(',').collect::<Vec<_>>();
    assert_eq!(hosts.len(), 3);
    for host in &hosts {
        assert!(
            docker(&["--host", host, "info", "--format", "{{.Name}}"])
                .await
                .starts_with("citadel-agent-compat-")
        );
    }
    let database = std::env::var("CITADEL_AGENT_TEST_DATABASE_URL").unwrap();
    citadel_database::MigrationRunner::migrate(&database)
        .await
        .unwrap();
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(12)
        .connect(&database)
        .await
        .unwrap();
    let store = Arc::new(PostgresBuildRepository::new(pool.clone()));
    let fixture = Fixture {
        root: std::env::temp_dir().join(format!("scaling-{}", Uuid::now_v7())),
        stop: CancellationToken::new(),
    };
    std::fs::create_dir_all(&fixture.root).unwrap();
    let registry = EdgeRegistry::default();
    let edge_store = PostgresEdgeStore::new(pool.clone());
    let intake = EdgeIntake::new(edge_store.clone(), registry.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let core_url = format!("http://{}", listener.local_addr().unwrap());
    let stop = fixture.stop.clone();
    let core = tokio::spawn(async move {
        tonic::transport::Server::builder()
            .add_service(EdgeAgentServiceServer::new(intake))
            .serve_with_incoming_shutdown(
                futures_util::stream::unfold(listener, |l| async {
                    Some((l.accept().await.map(|v| v.0), l))
                }),
                stop.cancelled_owned(),
            )
            .await
            .unwrap();
    });
    // Each Docker RUN blocks on this HTTP response. Six received requests prove
    // actual overlap without inferring it from durations or sleeping workloads.
    let listener = tokio::net::TcpListener::bind("0.0.0.0:0").await.unwrap();
    // Nested Docker build networks do not inherit outer Docker DNS aliases.
    // Use the runner's routed address, reachable from all three inner daemons.
    let route = tokio::net::TcpStream::connect(hosts[0].strip_prefix("tcp://").unwrap())
        .await
        .unwrap();
    let barrier_url = format!(
        "http://{}:{}",
        route.local_addr().unwrap().ip(),
        listener.local_addr().unwrap().port()
    );
    drop(route);
    let (arrived, mut arrivals) = tokio::sync::mpsc::channel(6);
    let (release, receiver) = tokio::sync::watch::channel(false);
    let stop = fixture.stop.clone();
    let barrier = tokio::spawn(async move {
        axum::serve(
            listener,
            axum::Router::new().route(
                "/ready/{index}",
                axum::routing::get(
                    move |axum::extract::Path(index): axum::extract::Path<usize>| {
                        let arrived = arrived.clone();
                        let mut receiver = receiver.clone();
                        async move {
                            arrived.send(index).await.unwrap();
                            while !*receiver.borrow_and_update() {
                                if receiver.changed().await.is_err() {
                                    break;
                                }
                            }
                            "released"
                        }
                    },
                ),
            ),
        )
        .with_graceful_shutdown(stop.cancelled_owned())
        .await
        .unwrap();
    });
    let repository = Uuid::now_v7();
    sqlx::query("INSERT INTO gitrepositories(id,createdbyactorid,defaultbranch,name,status,syncmode,url,controlstate) VALUES($1,$2,'main',$1::text,'Healthy','Manual','https://example.test/scaling.git','Idle')").bind(repository).bind(citadel_identity::SYSTEM_ACTOR_ID).execute(&pool).await.unwrap();
    let checkout = fixture.root.join(repository.to_string());
    std::fs::create_dir(&checkout).unwrap();
    // Leave room for tar headers within the existing 12 MiB archive limit.
    let payload_bytes = 12 * 1024 * 1024 - 64 * 1024;
    std::fs::write(checkout.join("payload"), vec![b'x'; payload_bytes]).unwrap();
    std::fs::write(checkout.join("Dockerfile"),format!("FROM compatibility-base:local\nCOPY payload /payload\nARG INDEX\nRUN test $(wc -c </payload) = {payload_bytes} && wget -qO- {barrier_url}/ready/$INDEX\n")).unwrap();
    for args in [
        vec!["init", "-b", "main"],
        vec!["add", "Dockerfile", "payload"],
        vec![
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.test",
            "commit",
            "-m",
            "scaling",
        ],
        vec!["update-ref", "refs/remotes/origin/main", "HEAD"],
    ] {
        git(&checkout, &args);
    }
    let mut agents = Vec::new();
    let mut wires = Vec::new();
    let mut executors = HashMap::<Uuid, Arc<dyn BuildExecutor>>::new();
    let actor =
        serde_json::from_value(serde_json::json!(citadel_identity::SYSTEM_ACTOR_ID)).unwrap();
    for (index, host) in hosts.iter().enumerate() {
        let platform = Uuid::now_v7();
        sqlx::query("INSERT INTO platforms(id,address,connectortype,cpucount,imagecount,memtotal,name,networkcount,platformdescriptor,status,volumecount) VALUES($1,$1::text,$2,0,0,0,$1::text,0,'{\"$type\":\"Docker\"}','Online',0)").bind(platform).bind(if index==0 {"Agent"}else{"EdgeAgent"}).execute(&pool).await.unwrap();
        let mut values = HashMap::from([
            ("CITADEL_AGENT_BUILD_SLOTS", "2".into()),
            ("CITADEL_AGENT_HEAVY_SLOTS", "2".into()),
            ("DOCKER_HOST", host.to_string()),
            (
                "CITADEL_CORE_PUBLIC_KEY",
                AgentRequestSigner::from_bytes(&[42; 32]).public_key_base64(),
            ),
        ]);
        if index > 0 {
            let token = edge_store
                .create_enrollment(
                    &EdgeTarget::platform(platform),
                    citadel_identity::SYSTEM_ACTOR_ID,
                )
                .await
                .unwrap()
                .1;
            values.extend([
                ("CITADEL_AGENT_MODE", "edge".into()),
                ("CITADEL_CORE_URL", core_url.clone()),
                ("CITADEL_EDGE_AGENT_PROFILE", "edge-agent".into()),
                ("CITADEL_EDGE_ENROLLMENT_TOKEN", token),
                (
                    "CITADEL_EDGE_AGENT_KEY_PATH",
                    fixture
                        .root
                        .join(format!("{index}.key"))
                        .display()
                        .to_string(),
                ),
                (
                    "CITADEL_EDGE_IDENTITY_PATH",
                    fixture
                        .root
                        .join(format!("{index}.identity"))
                        .display()
                        .to_string(),
                ),
            ]);
        }
        let (child, address) = start_agent(&values).await;
        agents.push(child);
        let executor: Arc<dyn BuildExecutor> = if index == 0 {
            let client = AgentClient::connect(
                &address,
                AgentRequestSigner::from_bytes(&[42; 32])
                    .for_agent("7fb4c244-485a-4b80-9611-b29e456adf65".parse().unwrap()),
                Duration::from_secs(30),
                true,
            )
            .await
            .unwrap();
            client.handshake(&fixture.stop).await.unwrap();
            wires.push(Wire::Direct(
                Channel::from_shared(address.clone())
                    .unwrap()
                    .connect()
                    .await
                    .unwrap(),
                "7fb4c244-485a-4b80-9611-b29e456adf65".into(),
            ));
            Arc::new(AgentDockerBuildExecutor::new(
                fixture.root.clone(),
                client,
                Arc::new(NoCredentials),
                Arc::new(NoCredentials),
                64 * 1024,
                store.clone(),
            ))
        } else {
            wires.push(Wire::Edge(
                current_session(&registry, &EdgeTarget::platform(platform), None).await,
            ));
            Arc::new(AgentDockerBuildExecutor::new_edge(
                fixture.root.clone(),
                registry.clone(),
                Arc::new(NoCredentials),
                Arc::new(NoCredentials),
                64 * 1024,
                store.clone(),
            ))
        };
        executors.insert(platform, executor.clone());
        for slot in 0..2 {
            // Two configured Direct platform aliases share the same daemon.
            let target = if index == 0 && slot == 1 {
                let alias = Uuid::now_v7();
                sqlx::query("INSERT INTO platforms SELECT (jsonb_populate_record(NULL::platforms,to_jsonb(p)||jsonb_build_object('id',$2::text,'address',$2::text,'name',$2::text))).* FROM platforms p WHERE p.id=$1").bind(platform).bind(alias).execute(&pool).await.unwrap();
                executors.insert(alias, executor.clone());
                alias
            } else {
                platform
            };
            let mut input:BuildProjectConfiguration=serde_json::from_value(serde_json::json!({"name":format!("scaling-{platform}-{slot}"),"enabled":true,"gitRepositoryId":repository,"branch":"main","platformId":target,"pushToRegistry":false,"buildArgs":[{"name":"INDEX","value":(index*2+slot).to_string()}],"timeoutSeconds":180})).unwrap();
            input.validate().unwrap();
            let project = store.create(actor, &input).await.unwrap();
            store.enqueue(actor, project.id, "Manual").await.unwrap();
        }
    }
    let preparation = Arc::new(tokio::sync::Semaphore::new(2));
    let logs = Arc::new(Logs::default());
    let executors = Arc::new(executors);
    let mut controllers = tokio::task::JoinSet::new();
    for _ in 0..6 {
        let (store, preparation, logs, executors, stop) = (
            store.clone(),
            preparation.clone(),
            logs.clone(),
            executors.clone(),
            fixture.stop.clone(),
        );
        controllers.spawn(async move {
            let (claim, permit) = tokio::time::timeout(Duration::from_secs(90), async {
                loop {
                    let permit = preparation.clone().acquire_owned().await.unwrap();
                    if let Some(claim) = store
                        .claim_next(chrono::Utc::now() - chrono::Duration::minutes(5))
                        .await
                        .unwrap()
                    {
                        break (claim, permit);
                    }
                    drop(permit);
                    tokio::time::sleep(Duration::from_millis(20)).await;
                }
            })
            .await
            .unwrap();
            let result = executors[&claim.project.platform_id.unwrap()]
                .execute_with_preparation(&claim, logs.as_ref(), &stop, Some(permit))
                .await;
            assert_eq!(
                result.status,
                BuildRunStatus::Succeeded,
                "{:?}",
                result.error_message
            );
            assert!(store.finish(&claim, &result, None).await.unwrap());
        });
    }
    let mut seen = std::collections::BTreeSet::new();
    tokio::time::timeout(Duration::from_secs(120), async {
        while seen.len() < 6 {
            tokio::select! {
                index = arrivals.recv() => { seen.insert(index.expect("barrier closed")); }
                early = controllers.join_next() => panic!("build ended before reaching overlap barrier: {early:?}"),
            }
        }
    })
    .await
    .expect("six actual Docker RUNs must overlap");
    assert_eq!(
        preparation.available_permits(),
        2,
        "execution must release Core preparation permits"
    );
    let counts: Vec<i64> =
        sqlx::query_scalar("SELECT count(*) FROM buildexecutorreservations GROUP BY daemonid")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(counts, vec![2, 2, 2]);
    // The same host's data governor cannot admit work over the combined limit.
    for wire in &wires {
        let host: CheckBuildHostResponse = wire
            .unary(
                Kind::ImageCheckBuildHost,
                "/citadel.images.v1.ImageService/CheckBuildHost",
                (),
            )
            .await;
        let refused: DataAdmissionResponse = wire
            .unary(
                Kind::ImageReserveData,
                "/citadel.images.v1.ImageService/ReserveData",
                DataOperationRequest {
                    run_id: Uuid::now_v7().to_string(),
                    attempt_id: Uuid::now_v7().to_string(),
                    operation_id: Uuid::now_v7().to_string(),
                    admission_authority_id: host.admission_authority_id,
                    docker_daemon_id: host.docker_daemon_id,
                    admission_generation: host.admission_generation,
                },
            )
            .await;
        assert_eq!(
            refused.admission,
            BuildAdmissionResult::BuildAdmissionCapacityBusyNotStarted as i32
        );
        let script_host: ScriptHost = wire
            .unary(
                Kind::ScriptCheckHost,
                "/citadel.scripts.v1.ScriptService/CheckHost",
                (),
            )
            .await;
        let mut scripts = wire
            .stream::<_, ScriptEvent>(
                Kind::ScriptRun,
                "/citadel.scripts.v1.ScriptService/Run",
                ScriptRequest {
                    attempt: Some(ScriptAttempt {
                        run_id: Uuid::now_v7().to_string(),
                        attempt_id: Uuid::now_v7().to_string(),
                        authority_id: script_host.authority_id,
                        generation: script_host.generation,
                    }),
                    source: "throw new Error('capacity refusal must precede execution');".into(),
                    token: "fixture-never-used".into(),
                    args_json: "{}".into(),
                    core_url: "https://core.invalid".into(),
                    timeout_seconds: 10,
                    maximum_log_bytes: 1024,
                },
            )
            .await;
        let refusal = tokio::time::timeout(Duration::from_secs(10), scripts.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert!(
            refusal.not_started,
            "builds must share heavy capacity with remote scripts"
        );
        assert!(scripts.next().await.is_none());
    }
    release.send(true).unwrap();
    tokio::time::timeout(Duration::from_secs(90), async {
        while let Some(result) = controllers.join_next().await {
            result.unwrap();
        }
    })
    .await
    .unwrap();
    assert!(logs.0.load(Ordering::SeqCst) > 0 && logs.0.load(Ordering::SeqCst) <= 6 * 64 * 1024);
    let remaining: i64 = sqlx::query_scalar("SELECT count(*) FROM buildexecutorreservations")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(remaining, 0);
    for child in &mut agents {
        child.kill().await.unwrap();
        child.wait().await.unwrap();
    }
    fixture.stop.cancel();
    core.await.unwrap();
    barrier.await.unwrap();
}
