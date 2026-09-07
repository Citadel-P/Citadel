use super::*;
use citadel_adapters::{
    agent::{AgentClient, AgentRequestSigner},
    backup_source_planner::PostgresBackupSourcePlanner,
    edge::{EdgeIntake, EdgeRegistry, EdgeTarget, PostgresEdgeStore},
};
use citadel_contracts::citadel::edge::v1::edge_agent_service_server::EdgeAgentServiceServer;
use citadel_platforms::{PlatformInventoryPort, PlatformRuntimePort};

const DIND: &str = "docker:27.5.1-dind";

#[path = "interactive.rs"]
mod interactive;
#[path = "node_setup.rs"]
mod node_setup;
#[path = "volume_content.rs"]
mod volume_content;

struct Cluster {
    nodes: Vec<String>,
    manager_agent: String,
    image_registry: String,
    sockets: String,
    directory: std::path::PathBuf,
    cancellation: CancellationToken,
    intake: Option<tokio::task::JoinHandle<()>>,
}

impl Cluster {
    fn new() -> Self {
        let suffix = Uuid::now_v7().simple().to_string();
        Self {
            nodes: (0..3)
                .map(|i| format!("citadel-backup-swarm-{suffix}-{i}"))
                .collect(),
            manager_agent: format!("citadel-backup-manager-agent-{suffix}"),
            image_registry: format!("citadel-backup-registry-{suffix}"),
            sockets: format!("citadel-backup-socket-{suffix}"),
            directory: std::env::temp_dir().join(format!("citadel-backup-swarm-{suffix}")),
            cancellation: CancellationToken::new(),
            intake: None,
        }
    }

    async fn node(&self, index: usize, arguments: &[&str], stdin: Option<&[u8]>) -> Vec<u8> {
        let mut args = vec!["exec"];
        if stdin.is_some() {
            args.push("-i");
        }
        args.extend([self.nodes[index].as_str(), "docker"]);
        args.extend_from_slice(arguments);
        docker(&args, stdin).await
    }

    async fn start(
        &mut self,
        pool: &sqlx::PgPool,
        platform: Uuid,
        registry: &EdgeRegistry,
    ) -> (AgentClient, Vec<String>) {
        let network = std::env::var("CITADEL_PHASE7_AGENT_NETWORK").unwrap();
        let image = std::env::var("CITADEL_PHASE7_AGENT_IMAGE").unwrap();
        let core_host = std::env::var("CITADEL_PHASE7_CORE_IP").unwrap();
        docker(
            &[
                "run",
                "--detach",
                "--name",
                &self.image_registry,
                "--network",
                &network,
                "registry@sha256:a3d8aaa63ed8681a604f1dea0aa03f100d5895b6a58ace528858a7b332415373",
            ],
            None,
        )
        .await;
        let registry_ip = String::from_utf8(
            docker(
                &[
                    "inspect",
                    "--format",
                    "{{range .NetworkSettings.Networks}}{{.IPAddress}}{{end}}",
                    &self.image_registry,
                ],
                None,
            )
            .await,
        )
        .unwrap()
        .trim()
        .to_owned();
        std::fs::create_dir(&self.directory).unwrap();
        let archive = self.directory.join("agent.tar");
        docker(
            &["save", "--output", archive.to_str().unwrap(), &image, IMAGE],
            None,
        )
        .await;
        for (index, node) in self.nodes.iter().enumerate() {
            let mut args = vec![
                "run",
                "--detach",
                "--privileged",
                "--name",
                node,
                "--network",
                &network,
                "--env",
                "DOCKER_TLS_CERTDIR=",
            ];
            let mount = format!("{}:/var/run", self.sockets);
            if index == 0 {
                args.extend(["--volume", &mount]);
            }
            args.extend([
                DIND,
                "--host=unix:///var/run/docker.sock",
                "--bip=10.223.0.1/24",
                "--insecure-registry=0.0.0.0/0",
            ]);
            docker(&args, None).await;
            let deadline = tokio::time::Instant::now() + Duration::from_secs(60);
            loop {
                let result = run(
                    ProcessRequest::new("docker").args(["exec", node, "docker", "info"]),
                    &CancellationToken::new(),
                )
                .await
                .unwrap();
                if result.succeeded() {
                    break;
                }
                assert!(
                    tokio::time::Instant::now() < deadline,
                    "Nested Docker did not start"
                );
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
            docker(
                &["cp", archive.to_str().unwrap(), &format!("{node}:/root/")],
                None,
            )
            .await;
            // dind mounts its own /tmp tmpfs after startup; Docker's archive API
            // writes below that mount. /root is visible to both cp and exec.
            self.node(index, &["load", "--input", "/root/agent.tar"], None)
                .await;
        }
        let ip = String::from_utf8(
            docker(
                &[
                    "inspect",
                    "--format",
                    "{{range .NetworkSettings.Networks}}{{.IPAddress}}{{end}}",
                    &self.nodes[0],
                ],
                None,
            )
            .await,
        )
        .unwrap()
        .trim()
        .to_owned();
        self.node(0, &["swarm", "init", "--advertise-addr", &ip], None)
            .await;
        let token = String::from_utf8(
            self.node(0, &["swarm", "join-token", "--quiet", "worker"], None)
                .await,
        )
        .unwrap();
        for i in 1..3 {
            self.node(
                i,
                &[
                    "swarm",
                    "join",
                    "--token",
                    token.trim(),
                    &format!("{ip}:2377"),
                ],
                None,
            )
            .await;
        }
        let mut ids = vec![];
        for i in 0..3 {
            ids.push(
                String::from_utf8(
                    self.node(i, &["info", "--format", "{{.Swarm.NodeID}}"], None)
                        .await,
                )
                .unwrap()
                .trim()
                .to_owned(),
            );
        }
        let mut key = [0; 32];
        getrandom::fill(&mut key).unwrap();
        let signer = AgentRequestSigner::from_bytes(&key);
        key.fill(0);
        docker(
            &[
                "run",
                "--detach",
                "--name",
                &self.manager_agent,
                "--network",
                &network,
                "--volume",
                &format!("{}:/var/run", self.sockets),
                "--env",
                &format!("HUB_PUBLIC_KEY={}", signer.public_key_base64()),
                "--env",
                "CITADEL_AGENT_TLS_MODE=Disabled",
                &image,
            ],
            None,
        )
        .await;
        let address = format!("http://{}:9000", self.manager_agent);
        let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
        let agent = loop {
            if let Ok(agent) =
                AgentClient::connect(&address, signer.clone(), Duration::from_secs(10), true).await
                && agent.handshake(&self.cancellation).await.is_ok()
            {
                break agent;
            }
            assert!(
                tokio::time::Instant::now() < deadline,
                "Manager Agent did not start"
            );
            tokio::time::sleep(Duration::from_millis(250)).await;
        };
        let info = agent.get_info(&self.cancellation).await.unwrap();
        let cluster = info.swarm.as_ref().unwrap().cluster_id.as_ref().unwrap();
        sqlx::query("INSERT INTO platforms(id,address,connectortype,cpucount,imagecount,memtotal,name,networkcount,platformdescriptor,status,volumecount,clusterid) VALUES($1,$2,'Agent',0,0,0,$3,0,$4,'Online',0,$5)")
            .bind(platform).bind(&address).bind(format!("backup-{platform}")).bind(json!({"$type":"DockerSwarm","nodeID":ids[0],"clusterId":cluster,"daemonId":info.daemon_id})).bind(cluster).execute(pool).await.unwrap();
        let listener = tokio::net::TcpListener::bind("0.0.0.0:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let incoming = futures_util::stream::unfold(listener, |listener| async {
            Some((listener.accept().await.map(|(socket, _)| socket), listener))
        });
        let intake = EdgeIntake::new(PostgresEdgeStore::new(pool.clone()), registry.clone());
        let cancel = self.cancellation.clone();
        self.intake = Some(tokio::spawn(async move {
            tonic::transport::Server::builder()
                .add_service(EdgeAgentServiceServer::new(intake))
                .serve_with_incoming_shutdown(incoming, cancel.cancelled_owned())
                .await
                .unwrap()
        }));
        node_setup::verify(
            self,
            pool,
            platform,
            registry,
            &agent,
            &format!("http://{core_host}:{port}"),
            &format!("{registry_ip}:5000"),
        )
        .await;
        (agent, ids)
    }

    async fn cleanup(mut self) {
        self.cancellation.cancel();
        if let Some(task) = self.intake.take() {
            let _ = tokio::time::timeout(Duration::from_secs(10), task).await;
        }
        for node in [&self.manager_agent, &self.image_registry]
            .into_iter()
            .chain(self.nodes.iter())
        {
            let _ = run(
                ProcessRequest::new("docker").args(["rm", "--force", "--volumes", node]),
                &CancellationToken::new(),
            )
            .await;
        }
        let _ = run(
            ProcessRequest::new("docker").args(["volume", "rm", &self.sockets]),
            &CancellationToken::new(),
        )
        .await;
        let _ = std::fs::remove_dir_all(&self.directory);
    }
}

#[tokio::test]
#[ignore = "requires Test-Phase7LocalBackup.ps1 -UseRustFs -AgentImage <candidate> -MultiNode"]
async fn worker_volume_backs_up_to_rustfs_and_restores_on_another_worker() {
    let database = std::env::var("CITADEL_PHASE7_LOCAL_BACKUP_DATABASE_URL").unwrap();
    MigrationRunner::migrate(&database).await.unwrap();
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&database)
        .await
        .unwrap();
    let mut cluster = Cluster::new();
    let platform = Uuid::now_v7();
    let registry = EdgeRegistry::default();
    let result=AssertUnwindSafe(async {
        let (agent,nodes)=cluster.start(&pool,platform,&registry).await;
        interactive::verify(&cluster,&pool,&agent,platform,&nodes,&registry).await;
        let source="citadel-source";let target="citadel-restored";
        for i in 0..3 {
            cluster.node(i,&["volume","create",source],None).await;
            cluster.node(i,&["run","--rm","-i","--volume",&format!("{source}:/fixture"),"--entrypoint","sh",IMAGE,"-c","cat > /fixture/payload.txt"],Some(if i==1 {PAYLOAD} else {b"wrong-node"})).await;
        }
        volume_content::verify(&cluster,&pool,&agent,platform,&nodes,&registry,source).await;
        let secret=Uuid::now_v7();
        for id in [secret,ACCESS_KEY_ID,SECRET_KEY_ID] {sqlx::query("INSERT INTO secretdefinitions(id,name,providertype) VALUES($1,$2,'InternalEncrypted')").bind(id).bind(format!("secret-{id}")).execute(&pool).await.unwrap();}
        let actor=ActorId::new(SYSTEM_ACTOR_ID);let store=PostgresBackupStore::new(pool.clone());
        let repository=store.create_repository(actor,&BackupRepositoryInput {name:format!("repo-{platform}"),description:None,password_secret_id:secret,spec:json!({"$type":"S3Compatible","endpoint":std::env::var("CITADEL_PHASE7_RUSTFS_ENDPOINT").unwrap(),"bucket":"citadel-backups","allowInsecureHttp":true,"accessKeySecretId":ACCESS_KEY_ID,"secretKeySecretId":SECRET_KEY_ID})}).await.unwrap();
        let executor=DockerResticBackupExecutor::new("/no-core-docker-allowed",IMAGE,Arc::new(Password),256*1024,pool.clone()).with_agent(Some(agent)).with_edge(registry.clone());
        let cancel=CancellationToken::new();
        executor.repository(&repository,"Initialize","Platform",Some(platform),&cancel).await.unwrap();
        let mut input=BackupPolicyInput {name:format!("policy-{platform}"),description:None,source:json!({"$type":"DockerVolume","platformId":platform,"dockerNodeId":nodes[1],"volumeName":source}),backup_repository_id:repository.id,enabled:true,cron:None,time_zone:None,webhook:None,keep_last_successful:Some(2),timeout_seconds:Some(120),alert_on_failure:false,run_as_actor_id:None,tag_ids:vec![]};input.validate(actor).unwrap();
        let policy=store.create_policy(actor,&input).await.unwrap();let run=store.enqueue_backup(actor,policy.id,"Manual").await.unwrap();
        let claim=store.claim_backup(Utc::now()-chrono::Duration::hours(1)).await.unwrap().unwrap();
        assert_eq!(run.id,claim.run.id);
        let plan=PostgresBackupSourcePlanner::new(pool.clone()).plan(&claim,&cancel).await.unwrap();
        assert_eq!(plan.items[0].docker_node_id.as_deref(),Some(nodes[1].as_str()));
        store.prepare_backup_items(&claim,&plan).await.unwrap();
        let completed=executor.backup(&claim,&plan,&cancel).await;store.finish_backup(&claim,&completed).await.unwrap();
        assert_eq!(completed.status,"Succeeded","error={:?}; warnings={:?}",completed.error_message,completed.warnings);
        let persisted=store.get_run(run.id).await.unwrap();assert_eq!(persisted.items[0].docker_node_id.as_deref(),Some(nodes[1].as_str()));
        let queued=store.enqueue_restore(BackupRestoreRequest {actor,backup_run_id:run.id,target_platform_id:platform,target_volume_name:target.into(),overwrite_existing:false,target_docker_node_id:Some(nodes[2].clone()),source_backup_run_item_id:Some(persisted.items[0].id)}).await.unwrap();
        let restore=store.claim_restore(Utc::now()-chrono::Duration::hours(1)).await.unwrap().unwrap();assert_eq!(restore.run.id,queued.id);
        let restored=executor.restore(&restore,&cancel).await;store.finish_restore(&restore,&restored).await.unwrap();assert_eq!(restored.status,"Succeeded","{:?}",restored.error_message);
        assert_eq!(cluster.node(2,&["run","--rm","--volume",&format!("{target}:/fixture:ro"),"--entrypoint","cat",IMAGE,"/fixture/payload.txt"],None).await,PAYLOAD);
        assert_eq!(store.get_restore(queued.id).await.unwrap().status,"Succeeded");
        for i in 0..3 {
            let helpers=cluster.node(i,&["ps","--all","--filter","label=com.citadel.system-role=backup-helper","--quiet"],None).await;
            assert!(helpers.is_empty(),"Backup helper leaked on node {i}");
        }
        // The source volume name also exists on the manager. Losing worker coverage
        // must fail, not back up that other volume with the same name.
        registry.get(&EdgeTarget::node(platform,nodes[1].clone())).unwrap().close();
        let failed=executor.backup(&claim,&plan,&cancel).await;
        assert_ne!(failed.status,"Succeeded");
    }).catch_unwind().await;
    cluster.cleanup().await;
    pool.close().await;
    if let Err(panic) = result {
        std::panic::resume_unwind(panic);
    }
}
