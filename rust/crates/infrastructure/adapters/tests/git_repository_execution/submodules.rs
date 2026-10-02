use super::*;
use axum::{
    Router,
    extract::Path,
    http::{HeaderMap, StatusCode},
    routing::get,
};
use base64::{Engine, engine::general_purpose::STANDARD};
use citadel_git::{CreateGitAccount, GitAuthConfiguration, GitAuthType, GitTransport};
use std::path::PathBuf;
use std::sync::Mutex;

async fn serve_repository(
    repository: PathBuf,
    authorization: Option<String>,
    received: Arc<Mutex<Vec<Option<String>>>>,
) -> (String, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let app = Router::new().route(
        "/contracts/{*path}",
        get(move |Path(path): Path<String>, headers: HeaderMap| {
            let repository = repository.clone();
            let authorization = authorization.clone();
            let received = received.clone();
            async move {
                let supplied = headers
                    .get("authorization")
                    .map(|value| value.to_str().unwrap().to_owned());
                received.lock().unwrap().push(supplied.clone());
                if supplied != authorization {
                    return (StatusCode::UNAUTHORIZED, Vec::new());
                }
                if path.split('/').any(|part| part == "..") {
                    return (StatusCode::BAD_REQUEST, Vec::new());
                }
                match tokio::fs::read(repository.join(path)).await {
                    Ok(bytes) => (StatusCode::OK, bytes),
                    Err(_) => (StatusCode::NOT_FOUND, Vec::new()),
                }
            }
        }),
    );
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    (origin, server)
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE7_DATABASE_URL and Git"]
async fn private_submodules_use_the_repository_account_without_leaking_to_other_origins() {
    let database_url = std::env::var("CITADEL_PHASE7_DATABASE_URL").unwrap();
    MigrationRunner::migrate(&database_url).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&database_url)
        .await
        .unwrap();
    let actor = Uuid::now_v7();
    sqlx::query("INSERT INTO actors(id,isenabled,type) VALUES($1,true,'System')")
        .bind(actor)
        .execute(&pool)
        .await
        .unwrap();
    let root = std::env::temp_dir().join(format!("citadel-submodules-{}", Uuid::now_v7()));
    let module = root.join("contracts");
    let source = root.join("source");
    let workspace = root.join("workspace");
    for directory in [&module, &source] {
        std::fs::create_dir_all(directory).unwrap();
        git(directory, &["init", "--initial-branch=main"]);
        git(directory, &["config", "user.name", "Citadel Test"]);
        git(directory, &["config", "user.email", "citadel@example.test"]);
    }
    std::fs::write(module.join("contract.proto"), "pinned schema").unwrap();
    git(&module, &["add", "."]);
    git(&module, &["commit", "-qm", "schema"]);
    let pinned = command_output(&module, &["rev-parse", "HEAD"]);
    std::fs::write(module.join("contract.proto"), "newer schema").unwrap();
    git(&module, &["commit", "-qam", "advance"]);
    git(&module, &["update-server-info"]);

    let token = "submodule-test-token";
    let authorization = format!("Basic {}", STANDARD.encode(format!("git:{token}")));
    let private_requests = Arc::new(Mutex::new(Vec::new()));
    let public_requests = Arc::new(Mutex::new(Vec::new()));
    let (origin, private_server) = serve_repository(
        module.join(".git"),
        Some(authorization.clone()),
        private_requests.clone(),
    )
    .await;
    let (other_origin, public_server) =
        serve_repository(module.join(".git"), None, public_requests.clone()).await;
    let remote = format!("{origin}/source");
    git(&source, &["remote", "add", "origin", &remote]);
    std::fs::write(source.join(".gitmodules"), format!(
        "[submodule \"contracts\"]\npath = contracts\nurl = ../contracts\n[submodule \"public\"]\npath = public\nurl = {other_origin}/contracts\n"
    )).unwrap();
    git(&source, &["add", ".gitmodules"]);
    for path in ["contracts", "public"] {
        git(
            &source,
            &[
                "update-index",
                "--add",
                "--cacheinfo",
                &format!("160000,{},{}", pinned.trim(), path),
            ],
        );
    }
    git(&source, &["commit", "-qm", "pin submodules"]);
    git(
        &root,
        &[
            "clone",
            "--shared",
            source.to_str().unwrap(),
            workspace.to_str().unwrap(),
        ],
    );
    let id = Uuid::now_v7();
    insert_repository(&pool, actor, id, &remote, "submodules").await;
    let accounts = Arc::new(GitAccountService::new(
        Arc::new(PostgresGitAccountRepository::new(pool.clone())),
        Arc::new(AesGcmSecretProtector::new(&[91_u8; 32]).unwrap()),
    ));
    let service = GitRepositoryExecutionService::new(
        Arc::new(PostgresGitRepositoryExecutionPersistence::new(pool.clone())),
        accounts.clone(),
        Arc::new(GitCli::new(
            Arc::new(citadel_processes::SystemProcess),
            std::sync::Arc::new(
                citadel_adapters::filesystem::git_workspace::LocalGitWorkspace::new(
                    citadel_runtime::DynamicTasks::new(tokio_util::sync::CancellationToken::new()),
                ),
            ),
            Duration::from_secs(20),
        )),
        root.join("cache"),
        Duration::from_secs(60),
    );
    // This source fails without credentials, just like a private hosted submodule.
    assert!(
        service
            .initialize_submodules(id, &workspace, &CancellationToken::new())
            .await
            .is_err()
    );
    private_requests.lock().unwrap().clear();
    let account = accounts
        .create(
            ActorId::new(actor),
            CreateGitAccount {
                name: format!("submodules-{}", id.simple()),
                domain: "127.0.0.1".into(),
                transport: GitTransport::Http,
                auth_type: GitAuthType::Token,
                configuration: GitAuthConfiguration::Token {
                    token: token.into(),
                },
            },
        )
        .await
        .unwrap();
    sqlx::query("UPDATE gitrepositories SET gitaccountid=$1 WHERE id=$2")
        .bind(account.id)
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
    service
        .initialize_submodules(id, &workspace, &CancellationToken::new())
        .await
        .unwrap();
    for path in ["contracts", "public"] {
        assert_eq!(
            std::fs::read_to_string(workspace.join(path).join("contract.proto")).unwrap(),
            "pinned schema"
        );
    }
    let authenticated = private_requests.lock().unwrap().clone();
    assert!(!authenticated.is_empty());
    assert!(
        authenticated
            .iter()
            .all(|value| value.as_ref() == Some(&authorization))
    );
    let anonymous = public_requests.lock().unwrap().clone();
    assert!(!anonymous.is_empty());
    assert!(anonymous.iter().all(Option::is_none));
    for path in [
        ".git/config",
        ".git/modules/contracts/config",
        ".git/modules/public/config",
    ] {
        let config = std::fs::read_to_string(workspace.join(path)).unwrap();
        assert!(!config.contains(token));
        assert!(!config.contains("extraHeader"));
    }
    private_server.abort();
    public_server.abort();
    sqlx::query("DELETE FROM gitrepositoryrefs WHERE gitrepositoryid=$1")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM gitrepositories WHERE id=$1")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
    accounts.delete(&[account.id]).await.unwrap();
    sqlx::query("DELETE FROM actors WHERE id=$1")
        .bind(actor)
        .execute(&pool)
        .await
        .unwrap();
    std::fs::remove_dir_all(root).unwrap();
}
