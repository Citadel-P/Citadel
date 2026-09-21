use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Method, Request, StatusCode},
};
use chrono::{Duration, Utc};
use citadel_adapters::{
    persistence::postgres::{
        automation::PostgresAutomationRepository,
        bindings::PostgresBindingRepository,
        git::{
            accounts::PostgresGitAccountRepository,
            repositories::PostgresGitRepositoryExecutionPersistence,
        },
        identity::authentication::store::{PostgresIdentityStore, StaticEntitlementService},
    },
    security::identity::{
        automation_token::IdentityAutomationRunTokenIssuer,
        crypto::{
            AesGcmSecretProtector, Argon2PasswordHasher, JwtSessionTokenCodec,
            OpaqueServiceAccountTokenCodec,
        },
    },
};
use citadel_automation::{AutomationRepository, AutomationRuntimeConfig, AutomationService};
use citadel_bindings::SecretService;
use citadel_database::MigrationRunner;
use citadel_git::{GitAccountService, GitCli, GitRepositoryExecutionService};
use citadel_identity::{
    ADMIN_ROLE_ID, ActorPrincipal, AuthenticatedPrincipalType, IdentityService,
    NoopServiceAccountLastUsedTracker, SYSTEM_ACTOR_ID, SystemClock,
};
use citadel_primitives::{ActorId, PermissionLevel, ResourceType};
use citadel_server::api::routes::{
    automation as automation_http,
    automation::AutomationHttpState,
    bindings, git_accounts as git_accounts_http,
    git_accounts::GitAccountsHttpState,
    git_repositories as git_repositories_http,
    git_repositories::{GitCatalogHttpState, GitRepositoriesHttpState},
    registries, tags, webhooks as webhooks_http,
    webhooks::WebhooksHttpState,
};
use serde_json::{Value, json};
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use tower::ServiceExt;
use uuid::Uuid;

#[path = "resources_http/workload_tags.rs"]
mod workload_tags;

#[tokio::test]
#[ignore = "requires CITADEL_PHASE5_DATABASE_URL"]
async fn metadata_endpoints_enforce_authorization_and_persist_complete_lifecycles() {
    let database_url = std::env::var("CITADEL_PHASE5_DATABASE_URL")
        .expect("CITADEL_PHASE5_DATABASE_URL is required for this fixture");
    MigrationRunner::migrate(&database_url).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&database_url)
        .await
        .unwrap();
    let identity = Arc::new(IdentityService::new(
        Arc::new(PostgresIdentityStore::new(pool.clone())),
        Arc::new(Argon2PasswordHasher::default()),
        Arc::new(
            JwtSessionTokenCodec::new(&[23_u8; 32], "fixture".into(), "fixture".into()).unwrap(),
        ),
        Arc::new(OpaqueServiceAccountTokenCodec),
        Arc::new(StaticEntitlementService::new(true)),
        Arc::new(SystemClock),
        Arc::new(NoopServiceAccountLastUsedTracker),
        Duration::minutes(15),
        Duration::days(30),
    ));
    let secret_protector = Arc::new(AesGcmSecretProtector::new(&[29_u8; 32]).unwrap());
    let resources = Arc::new(
        SecretService::new(
            Arc::new(PostgresBindingRepository::new(pool.clone())),
            secret_protector.clone(),
        )
        .with_secret_provider_tester(Arc::new(
            citadel_adapters::persistence::postgres::bindings::secret_resolver::PostgresSecretValueResolver::new(
                pool.clone(),
                secret_protector.clone(),
            )
            .unwrap(),
        )),
    );
    let git_accounts = Arc::new(GitAccountService::new(
        Arc::new(PostgresGitAccountRepository::new(pool.clone())),
        secret_protector,
    ));
    let git_cache = std::env::temp_dir().join(format!("citadel-phase7-{}", Uuid::now_v7()));
    let git_execution = Arc::new(GitRepositoryExecutionService::new(
        Arc::new(PostgresGitRepositoryExecutionPersistence::new(pool.clone())),
        Arc::clone(&git_accounts),
        Arc::new(GitCli::new(
            std::sync::Arc::new(citadel_processes::SystemProcess),
            std::time::Duration::from_secs(5),
        )),
        git_cache.clone(),
        std::time::Duration::from_secs(60),
    ));
    let cancellation = tokio_util::sync::CancellationToken::new();
    let automation_shutdown = tokio_util::sync::CancellationToken::new();
    let automation_tasks = citadel_runtime::DynamicTasks::new(automation_shutdown.clone());

    let automation = Arc::new(AutomationService::new(
        std::sync::Arc::new(citadel_processes::SystemProcess),
        Arc::new(
            citadel_server::tasks::automation::TrackedAutomationTasks::new(
                automation_tasks.clone(),
            ),
        ),
        automation_shutdown.clone(),
        Arc::new(PostgresAutomationRepository::new(pool.clone())),
        Arc::new(IdentityAutomationRunTokenIssuer::new(Arc::clone(&identity))),
        AutomationRuntimeConfig {
            deno_path: "deno".into(),
            work_root: std::env::temp_dir()
                .join(format!("citadel-automation-http-{}", Uuid::now_v7())),
            internal_base_url: "http://127.0.0.1:8000".to_owned(),
            endpoint_catalog_json: citadel_server::automation_endpoint_catalog_json(),
            maximum_log_bytes: 64 * 1024,
            stale_after: std::time::Duration::from_secs(60),
        },
    ));
    let app = tags::router(tags::TagsHttpState {
        identity: Arc::clone(&identity),
        tags: Arc::new(citadel_adapters::persistence::postgres::tags::PostgresTagRepository::new(pool.clone())),
        realtime: None,
    })
    .merge(registries::router(registries::RegistriesHttpState {
        identity: Arc::clone(&identity),
        registries: Arc::new(
            citadel_adapters::persistence::postgres::registries::PostgresRegistryRepository::new(pool.clone()),
        ),
        realtime: None,
    }))
    .merge(bindings::router(bindings::BindingsHttpState {
        identity: Arc::clone(&identity),
        secrets: Arc::clone(&resources),
        realtime: None,
    }))
    .merge(git_repositories_http::catalog_router(GitCatalogHttpState {
        identity: Arc::clone(&identity),
        git_repositories: Arc::new(
            citadel_adapters::persistence::postgres::git::repositories::PostgresGitRepositoryPersistence::new(
                pool.clone(),
            ),
        ),
        realtime: None,
    }))
    .merge(git_accounts_http::router(GitAccountsHttpState {
        identity: Arc::clone(&identity),
        accounts: git_accounts,
        realtime: None,
    }))
    .merge(git_repositories_http::router(GitRepositoriesHttpState {
        identity: Arc::clone(&identity),
        repository: Arc::new(
            citadel_adapters::persistence::postgres::git::repositories::PostgresGitRepositoryPersistence::new(
                pool.clone(),
            ),
        ),
        execution: Arc::clone(&git_execution),
        realtime: None,
        cancellation,
    }))
    .merge(webhooks_http::router(WebhooksHttpState {
        git: Arc::clone(&git_execution),
        automation: Arc::clone(&automation),
        backups: None,
        builds: None,
        stacks: None,
        services: None,
        audit: None,
        alerts: None,
    }))
    .merge(automation_http::router(AutomationHttpState {
        identity,
        automation,
    }));
    assert_eq!(
        request(&app, Method::GET, "/api/v1/tags", None, None)
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );

    let administrator_actor_id = Uuid::now_v7();
    let administrator_user_id = Uuid::now_v7();
    let administrator_name = format!("phase5-admin-{}", administrator_user_id.simple());
    let mut transaction = pool.begin().await.unwrap();
    sqlx::query("INSERT INTO actors (id, isenabled, type) VALUES ($1, TRUE, 'User')")
        .bind(administrator_actor_id)
        .execute(&mut *transaction)
        .await
        .unwrap();
    sqlx::query("INSERT INTO users (id, actorid, createdat, createdbyactorid, email, name) VALUES ($1, $2, $3, $4, $5, $6)")
        .bind(administrator_user_id)
        .bind(administrator_actor_id)
        .bind(Utc::now())
        .bind(SYSTEM_ACTOR_ID)
        .bind(format!("{administrator_name}@example.test"))
        .bind(&administrator_name)
        .execute(&mut *transaction)
        .await
        .unwrap();
    sqlx::query("INSERT INTO actorroles (actorid, roleid) VALUES ($1, $2)")
        .bind(administrator_actor_id)
        .bind(ADMIN_ROLE_ID)
        .execute(&mut *transaction)
        .await
        .unwrap();
    transaction.commit().await.unwrap();
    let administrator = ActorPrincipal {
        subject_id: administrator_user_id,
        actor_id: ActorId::new(administrator_actor_id),
        name: administrator_name,
        principal_type: AuthenticatedPrincipalType::User,
        credential_id: None,
        roles: vec!["Admin".into()],
    };
    let suffix = Uuid::now_v7().simple().to_string();

    // Port AutomationActionTags_ShouldCreateFilterByTagNameAndReplace.
    let mut action_tags = Vec::new();
    for color in ["blue", "green"] {
        let response = request(
            &app,
            Method::POST,
            "/api/v1/tags",
            Some(administrator.clone()),
            Some(json!({"name":format!("automation-{color}-{suffix}"),"color":"#3366FF"})),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        action_tags.push(response_json(response).await);
    }

    workload_tags::verify(&app, &pool, &administrator, &action_tags).await;

    let automation_response = request(
        &app,
        Method::POST,
        "/api/v1/automation/actions",
        Some(administrator.clone()),
        Some(json!({
            "name":format!("phase7-action-{suffix}"),
            "description":"integration action",
            "code":"console.log(args);",
            "defaultArgsJson":"{}",
            "enabled":true,
            "scheduleEnabled":false,
            "scheduleCron":null,
            "scheduleTimeZone":"UTC",
            "webhook":null,
            "timeoutSeconds":30,
            "alertOnFailure":true,
            "runAsActorId":administrator_actor_id,
            "tagIds":[action_tags[0]["id"]]
        })),
    )
    .await;
    assert_eq!(automation_response.status(), StatusCode::OK);
    let automation_action = response_json(automation_response).await;
    let automation_id = automation_action["id"].as_str().unwrap();
    let automation_uuid = Uuid::parse_str(automation_id).unwrap();
    assert_eq!(automation_action["tags"][0]["id"], action_tags[0]["id"]);
    assert_eq!(automation_action["capabilities"]["canExecute"], true);
    assert!(automation_action["latestRun"].is_null());
    // Paid triggers cannot be enabled by an
    // unlicensed administrator; ordinary manual Actions remain available.
    assert_eq!(
        request(
            &app,
            Method::PATCH,
            &format!("/api/v1/automation/actions/{automation_id}"),
            Some(administrator.clone()),
            Some(
                json!({"scheduleEnabled":true,"scheduleCron":"* * * * *","scheduleTimeZone":"UTC"})
            )
        )
        .await
        .status(),
        StatusCode::FORBIDDEN
    );
    let schedule: bool = sqlx::query_scalar("SELECT scheduleenabled FROM actions WHERE id=$1")
        .bind(automation_uuid)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(!schedule);
    for (index, expected) in [(0, 1), (1, 0)] {
        let response = request(
            &app,
            Method::GET,
            &format!(
                "/api/v1/automation/actions?tags={}",
                action_tags[index]["name"].as_str().unwrap()
            ),
            Some(administrator.clone()),
            None,
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response_json(response).await["actions"]
                .as_array()
                .unwrap()
                .len(),
            expected
        );
    }
    let tags_path = format!("/api/v1/automation/actions/{automation_id}/tags");
    let response = request(
        &app,
        Method::PUT,
        &tags_path,
        Some(administrator.clone()),
        Some(json!({"tagIds":[action_tags[1]["id"]]})),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response_json(response).await["tags"][0]["id"],
        action_tags[1]["id"]
    );
    let response = request(
        &app,
        Method::PUT,
        &tags_path,
        Some(administrator.clone()),
        Some(json!({"tagIds":[Uuid::now_v7()]})),
    )
    .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let response = request(
        &app,
        Method::GET,
        &tags_path,
        Some(administrator.clone()),
        None,
    )
    .await;
    assert_eq!(
        response_json(response).await["tags"][0]["id"],
        action_tags[1]["id"]
    );
    let metadata_response = request(
        &app,
        Method::PATCH,
        &format!("/api/v1/automation/actions/{automation_id}/_metadata"),
        Some(administrator.clone()),
        Some(json!({"description":"metadata only"})),
    )
    .await;
    assert_eq!(metadata_response.status(), StatusCode::OK);
    assert_eq!(
        response_json(metadata_response).await["description"],
        "metadata only"
    );
    let activity_count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM activityevents WHERE resourceid=$1")
            .bind(automation_uuid)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        activity_count, 1,
        "metadata must not add a configuration activity"
    );
    let renamed = request(
        &app,
        Method::POST,
        "/api/v1/automation/actions/rename",
        Some(administrator.clone()),
        Some(json!({"id":automation_id,"name":format!("phase7-renamed-{suffix}")})),
    )
    .await;
    assert_eq!(renamed.status(), StatusCode::OK);
    let updated = request(
        &app,
        Method::PATCH,
        &format!("/api/v1/automation/actions/{automation_id}"),
        Some(administrator.clone()),
        Some(json!({"description":null,"timeoutSeconds":45})),
    )
    .await;
    assert_eq!(updated.status(), StatusCode::OK);
    let updated = response_json(updated).await;
    assert_eq!(updated["timeoutSeconds"], 45);
    assert!(updated["description"].is_null());
    let unsupported_update = request(
        &app,
        Method::PATCH,
        &format!("/api/v1/automation/actions/{automation_id}"),
        Some(administrator.clone()),
        Some(json!({"name":"bypass-rename"})),
    )
    .await;
    assert_eq!(unsupported_update.status(), StatusCode::BAD_REQUEST);
    // Keep this fixture's run queued for the cancellation endpoint. Actual HTTP
    // execution/progress is covered with real Deno by automation_http_execution.
    let queued = PostgresAutomationRepository::new(pool.clone())
        .enqueue(
            administrator.actor_id,
            automation_uuid,
            "Manual",
            &json!({"mode":"manual"}),
            None,
        )
        .await
        .unwrap();
    let run_id = queued.id.to_string();
    let persisted_run = response_json(
        request(
            &app,
            Method::GET,
            &format!("/api/v1/automation/actions/{automation_id}/runs/{run_id}"),
            Some(administrator.clone()),
            None,
        )
        .await,
    )
    .await;
    assert_eq!(persisted_run["argsJson"], "{\"mode\":\"manual\"}");
    assert_eq!(
        request(
            &app,
            Method::POST,
            &format!("/api/v1/automation/actions/{automation_id}/runs/{run_id}/cancel"),
            Some(administrator.clone()),
            None,
        )
        .await
        .status(),
        StatusCode::NO_CONTENT
    );
    let deleted = request(
        &app,
        Method::DELETE,
        &format!("/api/v1/automation/actions/{automation_id}"),
        Some(administrator.clone()),
        None,
    )
    .await;
    assert_eq!(deleted.status(), StatusCode::NO_CONTENT);
    let events: Vec<(String, Value)> = sqlx::query_as(
        "SELECT eventtype,info::jsonb FROM activityevents WHERE resourceid=$1 ORDER BY createdat,id",
    )
    .bind(automation_uuid)
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(
        events
            .iter()
            .map(|(kind, _)| kind.as_str())
            .collect::<Vec<_>>(),
        [
            "ActionCreated",
            "ActionRenamed",
            "ActionUpdated",
            "ActionRunQueued",
            "ActionRunCancelled",
            "ActionDeleted"
        ]
    );
    assert_eq!(events[1].1["NewName"], format!("phase7-renamed-{suffix}"));
    assert_eq!(events[2].1["NewAction"]["TimeoutSeconds"], 45);
    assert_eq!(events[4].1["RunId"], run_id);
    let malformed = request_raw(
        &app,
        Method::POST,
        "/api/v1/tags",
        Some(administrator.clone()),
        "{".into(),
    )
    .await;
    assert_eq!(malformed.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        malformed
            .headers()
            .get("content-type")
            .and_then(|value| value.to_str().ok()),
        Some("application/problem+json")
    );

    let git_account_response = request(
        &app,
        Method::POST,
        "/api/v1/gitAccounts",
        Some(administrator.clone()),
        Some(json!({
            "name":format!("account-{suffix}"),
            "domain":"git.example.test",
            "transport":"Https",
            "authType":"Token",
            "configuration":{"$type":"Token","token":"phase7-secret-token"}
        })),
    )
    .await;
    assert_eq!(git_account_response.status(), StatusCode::OK);
    let git_account = response_json(git_account_response).await;
    let git_account_id = git_account["id"].as_str().unwrap();
    assert!(git_account.get("configuration").is_none());
    let basic_git_account = response_json(
        request(
            &app,
            Method::POST,
            "/api/v1/gitAccounts",
            Some(administrator.clone()),
            Some(json!({
                "name":format!("basic-{suffix}"),
                "domain":"github.com",
                "transport":"Https",
                "authType":"Basic",
                "configuration":{
                    "$type":"Basic",
                    "username":"phase7-user",
                    "password":"phase7-password"
                }
            })),
        )
        .await,
    )
    .await;
    let basic_git_account_id = basic_git_account["id"].as_str().unwrap();
    assert_eq!(basic_git_account["authType"], "Basic");
    let ssh_git_account = response_json(
        request(
            &app,
            Method::POST,
            "/api/v1/gitAccounts",
            Some(administrator.clone()),
            Some(json!({
                "name":format!("ssh-{suffix}"),
                "domain":"github.com",
                "transport":"Ssh",
                "authType":"SshKey",
                "configuration":{
                    "$type":"SshKey",
                    "username":"git",
                    "privateKey":"-----BEGIN OPENSSH PRIVATE KEY-----phase7"
                }
            })),
        )
        .await,
    )
    .await;
    let ssh_git_account_id = ssh_git_account["id"].as_str().unwrap();
    assert_eq!(ssh_git_account["transport"], "Ssh");
    for invalid in [
        json!({
            "name":"",
            "domain":"github.com",
            "transport":"Https",
            "authType":"Token",
            "configuration":{"$type":"Token","token":"phase7-token"}
        }),
        json!({
            "name":format!("empty-token-{suffix}"),
            "domain":"github.com",
            "transport":"Https",
            "authType":"Token",
            "configuration":{"$type":"Token","token":""}
        }),
        json!({
            "name":format!("mismatch-{suffix}"),
            "domain":"github.com",
            "transport":"Https",
            "authType":"Basic",
            "configuration":{
                "$type":"SshKey",
                "username":"git",
                "privateKey":"-----BEGIN OPENSSH PRIVATE KEY-----phase7"
            }
        }),
    ] {
        assert_eq!(
            request(
                &app,
                Method::POST,
                "/api/v1/gitAccounts",
                Some(administrator.clone()),
                Some(invalid),
            )
            .await
            .status(),
            StatusCode::BAD_REQUEST
        );
    }
    let persisted_configuration: Value =
        sqlx::query_scalar("SELECT configuration FROM gitaccounts WHERE id=$1")
            .bind(Uuid::parse_str(git_account_id).unwrap())
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(persisted_configuration.get("$protected").is_some());
    assert!(
        !persisted_configuration
            .to_string()
            .contains("phase7-secret-token")
    );
    let git_account_config = response_json(
        request(
            &app,
            Method::GET,
            &format!("/api/v1/gitAccounts/{git_account_id}/_cfg"),
            Some(administrator.clone()),
            None,
        )
        .await,
    )
    .await;
    assert_eq!(
        git_account_config["configuration"]["token"],
        "phase7-secret-token"
    );
    let ssh_patch = response_json(
        request(
            &app,
            Method::PATCH,
            &format!("/api/v1/gitAccounts/{git_account_id}"),
            Some(administrator.clone()),
            Some(json!({
                "domain":"gitlab.com",
                "transport":"Ssh",
                "authType":"SshKey",
                "configuration":{
                    "$type":"SshKey",
                    "username":"git",
                    "privateKey":"-----BEGIN OPENSSH PRIVATE KEY-----patched"
                }
            })),
        )
        .await,
    )
    .await;
    assert_eq!(ssh_patch["domain"], "gitlab.com");
    assert_eq!(ssh_patch["transport"], "Ssh");
    assert_eq!(ssh_patch["authType"], "SshKey");
    assert_eq!(
        response_json(
            request(
                &app,
                Method::GET,
                &format!("/api/v1/gitAccounts/{git_account_id}/_cfg"),
                Some(administrator.clone()),
                None,
            )
            .await,
        )
        .await["configuration"]["privateKey"],
        "-----BEGIN OPENSSH PRIVATE KEY-----patched"
    );
    assert_eq!(
        request(
            &app,
            Method::PATCH,
            &format!("/api/v1/gitAccounts/{git_account_id}"),
            Some(administrator.clone()),
            Some(json!({"name":""})),
        )
        .await
        .status(),
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        request(
            &app,
            Method::PATCH,
            &format!("/api/v1/gitAccounts/{git_account_id}"),
            Some(administrator.clone()),
            Some(json!({
                "transport":"Https",
                "authType":"Basic",
                "configuration":{
                    "$type":"SshKey",
                    "username":"git",
                    "privateKey":"-----BEGIN OPENSSH PRIVATE KEY-----invalid"
                }
            })),
        )
        .await
        .status(),
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        request(
            &app,
            Method::PATCH,
            &format!("/api/v1/gitAccounts/{git_account_id}"),
            Some(administrator.clone()),
            Some(json!({
                "domain":"git.example.test",
                "transport":"Https",
                "authType":"Token",
                "configuration":{"$type":"Token","token":"phase7-secret-token"}
            })),
        )
        .await
        .status(),
        StatusCode::OK
    );
    let renamed_git_account = response_json(
        request(
            &app,
            Method::PATCH,
            &format!("/api/v1/gitAccounts/{git_account_id}"),
            Some(administrator.clone()),
            Some(json!({"name":format!("renamed-{suffix}")})),
        )
        .await,
    )
    .await;
    assert_eq!(renamed_git_account["name"], format!("renamed-{suffix}"));
    let preserved_git_account_config = response_json(
        request(
            &app,
            Method::GET,
            &format!("/api/v1/gitAccounts/{git_account_id}/_cfg"),
            Some(administrator.clone()),
            None,
        )
        .await,
    )
    .await;
    assert_eq!(
        preserved_git_account_config["configuration"]["token"],
        "phase7-secret-token"
    );
    assert_eq!(
        request(
            &app,
            Method::PATCH,
            &format!("/api/v1/gitAccounts/{git_account_id}"),
            Some(administrator.clone()),
            Some(json!({"name":format!("basic-{suffix}")})),
        )
        .await
        .status(),
        StatusCode::CONFLICT
    );
    let missing_git_account_id = Uuid::now_v7();
    assert_eq!(
        request(
            &app,
            Method::PATCH,
            &format!("/api/v1/gitAccounts/{missing_git_account_id}"),
            Some(administrator.clone()),
            Some(json!({"name":format!("missing-{suffix}")})),
        )
        .await
        .status(),
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        request(
            &app,
            Method::DELETE,
            "/api/v1/gitAccounts",
            Some(administrator.clone()),
            Some(json!({"ids":[missing_git_account_id]})),
        )
        .await
        .status(),
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        request(
            &app,
            Method::POST,
            "/api/v1/gitAccounts",
            Some(administrator.clone()),
            Some(json!({
                "name":format!("renamed-{suffix}"),
                "domain":"git.example.test",
                "transport":"Https",
                "authType":"Token",
                "configuration":{"$type":"Token","token":"another-token"}
            })),
        )
        .await
        .status(),
        StatusCode::CONFLICT
    );

    let tag_response = request(
        &app,
        Method::POST,
        "/api/v1/tags",
        Some(administrator.clone()),
        Some(json!({"name":format!("http-{suffix}"),"color":"#334455"})),
    )
    .await;
    assert_eq!(tag_response.status(), StatusCode::OK);
    let tag = response_json(tag_response).await;
    let tag_id = tag["id"].as_str().unwrap();

    let registry_response = request(
        &app,
        Method::POST,
        "/api/v1/registries",
        Some(administrator.clone()),
        Some(json!({
            "name":format!("registry-{suffix}"),
            "registryHost":"registry.example.test",
            "status":"Active",
            "configuration":{"$type":"Custom","Username":"user","Password":"password"},
            "tagIds":[tag_id]
        })),
    )
    .await;
    assert_eq!(registry_response.status(), StatusCode::OK);
    let registry = response_json(registry_response).await;
    let registry_id = registry["id"].as_str().unwrap();
    assert!(registry.get("configuration").is_none());

    let repository_response = request(
        &app,
        Method::POST,
        "/api/v1/gitRepositories",
        Some(administrator.clone()),
        Some(json!({
            "name":format!("repository-{suffix}"),
            "url":"https://git.example.test/team/repository.git",
            "defaultBranch":"main",
            "gitAccountId":git_account_id,
            "syncMode":"Manual",
            "webhook":{
                "enabled":true,
                "provider":"Generic",
                "authScheme":"BearerToken",
                "secret":"phase7-shared-secret",
                "branchFilter":"main"
            },
            "tagIds":[tag_id]
        })),
    )
    .await;
    assert_eq!(repository_response.status(), StatusCode::OK);
    let repository = response_json(repository_response).await;
    let repository_id = repository["id"].as_str().unwrap();
    let refs = response_json(
        request(
            &app,
            Method::GET,
            &format!("/api/v1/gitRepositories/{repository_id}/refs"),
            Some(administrator.clone()),
            None,
        )
        .await,
    )
    .await;
    assert_eq!(refs["refs"][0]["branch"], "main");
    assert_eq!(refs["refs"][0]["status"], "Pending");

    let actor_id = Uuid::now_v7();
    sqlx::query("INSERT INTO actors (id, isenabled, type) VALUES ($1, TRUE, 'User')")
        .bind(actor_id)
        .execute(&pool)
        .await
        .unwrap();
    for (resource_type, resource_id, permission) in [
        (
            ResourceType::Tag,
            Uuid::parse_str(tag_id).unwrap(),
            PermissionLevel::Read,
        ),
        (
            ResourceType::Registry,
            Uuid::parse_str(registry_id).unwrap(),
            PermissionLevel::Execute,
        ),
        (
            ResourceType::GitRepository,
            Uuid::parse_str(repository_id).unwrap(),
            PermissionLevel::Execute,
        ),
        (
            ResourceType::GitAccount,
            Uuid::parse_str(git_account_id).unwrap(),
            PermissionLevel::Read,
        ),
    ] {
        sqlx::query(
            "INSERT INTO resourceaccesses (id,actorid,permissionlevel,resourceid,resourcetype,specificpermissions) VALUES ($1,$2,$3,$4,$5,0)",
        )
        .bind(Uuid::now_v7())
        .bind(actor_id)
        .bind(permission as i32)
        .bind(resource_id)
        .bind(resource_type as i32)
        .execute(&pool)
        .await
        .unwrap();
    }
    let resource_actor = ActorPrincipal {
        subject_id: actor_id,
        actor_id: ActorId::new(actor_id),
        name: "resource operator".into(),
        principal_type: AuthenticatedPrincipalType::User,
        credential_id: None,
        roles: Vec::new(),
    };

    let sync_response = request(
        &app,
        Method::POST,
        &format!("/api/v1/gitRepositories/{repository_id}/sync?branch=main"),
        Some(resource_actor.clone()),
        None,
    )
    .await;
    assert_eq!(sync_response.status(), StatusCode::OK);
    let queued: (String, String) =
        sqlx::query_as("SELECT status,controlstate FROM gitrepositories WHERE id=$1")
            .bind(Uuid::parse_str(repository_id).unwrap())
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(queued, ("Pending".to_owned(), "Queued".to_owned()));

    let webhook_response = request_with_header(
        &app,
        Method::POST,
        &format!("/listener/generic/repo/{repository_id}/pull"),
        "authorization",
        "Bearer phase7-shared-secret",
        Some(json!({})),
    )
    .await;
    assert_eq!(webhook_response.status(), StatusCode::ACCEPTED);
    let repository_uuid = Uuid::parse_str(repository_id).unwrap();
    let version_before: i64 =
        sqlx::query_scalar("SELECT rowversion FROM gitrepositories WHERE id=$1")
            .bind(repository_uuid)
            .fetch_one(&pool)
            .await
            .unwrap();
    for payload in [
        json!({"branch":"unrelated"}),
        json!({"repository":"https://example.test/not-this-repository.git"}),
    ] {
        let ignored = request_with_header(
            &app,
            Method::POST,
            &format!("/listener/generic/repo/{repository_id}/pull"),
            "authorization",
            "Bearer phase7-shared-secret",
            Some(payload),
        )
        .await;
        assert_eq!(ignored.status(), StatusCode::ACCEPTED);
        assert_eq!(response_json(ignored).await["status"], "noop");
        let version: i64 = sqlx::query_scalar("SELECT rowversion FROM gitrepositories WHERE id=$1")
            .bind(repository_uuid)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(
            version, version_before,
            "ignored delivery cannot enqueue or change repository state"
        );
    }
    assert_eq!(
        request(
            &app,
            Method::POST,
            &format!("/listener/generic/repo/{repository_id}/pull"),
            None,
            Some(json!({})),
        )
        .await
        .status(),
        StatusCode::UNAUTHORIZED
    );

    let listed_tags = response_json(
        request(
            &app,
            Method::GET,
            "/api/v1/tags",
            Some(resource_actor.clone()),
            None,
        )
        .await,
    )
    .await;
    assert_eq!(listed_tags["tags"].as_array().unwrap().len(), 1);
    let listed_registries = response_json(
        request(
            &app,
            Method::GET,
            "/api/v1/registries?includeDisabled=true",
            Some(resource_actor.clone()),
            None,
        )
        .await,
    )
    .await;
    assert_eq!(listed_registries["registries"].as_array().unwrap().len(), 1);
    assert_eq!(
        listed_registries["registries"][0]["capabilities"]["canExecute"],
        true
    );
    let listed_git_accounts = response_json(
        request(
            &app,
            Method::GET,
            "/api/v1/gitAccounts",
            Some(resource_actor.clone()),
            None,
        )
        .await,
    )
    .await;
    let visible_git_accounts = listed_git_accounts["gitAccounts"].as_array().unwrap();
    assert_eq!(visible_git_accounts.len(), 1);
    assert_eq!(visible_git_accounts[0]["id"], git_account_id);
    assert_eq!(visible_git_accounts[0]["capabilities"]["canRead"], true);
    assert_eq!(
        request(
            &app,
            Method::PATCH,
            &format!("/api/v1/registries/{registry_id}"),
            Some(resource_actor.clone()),
            Some(json!({"description":"resource-scoped update"})),
        )
        .await
        .status(),
        StatusCode::OK
    );
    let repository_detail = response_json(
        request(
            &app,
            Method::GET,
            &format!("/api/v1/gitRepositories/{repository_id}"),
            Some(resource_actor.clone()),
            None,
        )
        .await,
    )
    .await;
    assert_eq!(
        repository_detail["latestActivityView"]["eventType"],
        "GitRepoCreated"
    );

    let binding_response = request(
        &app,
        Method::POST,
        "/api/v1/resourceBindings/global",
        Some(administrator.clone()),
        Some(json!({"name":format!("REGION_{suffix}").to_uppercase(),"kind":"Variable","value":"eu-west"})),
    )
    .await;
    assert_eq!(binding_response.status(), StatusCode::OK);
    let binding = response_json(binding_response).await;
    assert!(
        binding["entries"]
            .as_array()
            .unwrap()
            .iter()
            .any(
                |entry| entry["name"] == format!("REGION_{suffix}").to_uppercase()
                    && entry["value"] == "eu-west"
            )
    );

    let internal_secret_response = request(
        &app,
        Method::POST,
        "/api/v1/resourceBindings/secrets",
        Some(administrator.clone()),
        Some(json!({"name":format!("INTERNAL_{suffix}").to_uppercase(),"value":"protected-value"})),
    )
    .await;
    assert_eq!(internal_secret_response.status(), StatusCode::OK);
    let internal_secret = response_json(internal_secret_response).await;
    let internal_secret_id = internal_secret["id"].as_str().unwrap();

    let provider_response = request(
        &app,
        Method::POST,
        "/api/v1/resourceBindings/secret-providers/vault-kv2",
        Some(administrator.clone()),
        Some(json!({
            "name":format!("provider-{suffix}"),
            "address":"https://vault.example.test/",
            "mountPath":"/secret/",
            "token":"provider-token"
        })),
    )
    .await;
    assert_eq!(provider_response.status(), StatusCode::OK);
    let provider = response_json(provider_response).await;
    let provider_id = provider["id"].as_str().unwrap();
    // Vault connection/reference commands require Binding.Write even though
    // they do not persist anything. Authorization must precede provider access.
    for uri in [
        "/api/v1/resourceBindings/secret-providers/vault-kv2/test",
        "/api/v1/resourceBindings/secrets/external/test",
    ] {
        assert_eq!(
            request(&app, Method::POST, uri, None, Some(json!({})))
                .await
                .status(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            request(
                &app,
                Method::POST,
                uri,
                Some(resource_actor.clone()),
                Some(json!({}))
            )
            .await
            .status(),
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            request(
                &app,
                Method::POST,
                uri,
                Some(administrator.clone()),
                Some(json!({}))
            )
            .await
            .status(),
            StatusCode::BAD_REQUEST
        );
    }
    let no_token = request(
        &app,
        Method::POST,
        "/api/v1/resourceBindings/secret-providers/vault-kv2/test",
        Some(administrator.clone()),
        Some(json!({"address":"https://vault.example.test","mountPath":"secret"})),
    )
    .await;
    assert_eq!(no_token.status(), StatusCode::OK);
    assert_eq!(response_json(no_token).await["success"], false);
    let missing_provider = request(
        &app,
        Method::POST,
        "/api/v1/resourceBindings/secrets/external/test",
        Some(administrator.clone()),
        Some(json!({"providerId":Uuid::now_v7(),"externalPath":"apps/test","externalKey":"token"})),
    )
    .await;
    assert_eq!(missing_provider.status(), StatusCode::NOT_FOUND);
    assert_eq!(provider["address"], "https://vault.example.test");
    assert!(provider.get("token").is_none());

    let provider_patch = request(
        &app,
        Method::PATCH,
        &format!("/api/v1/resourceBindings/secret-providers/vault-kv2/{provider_id}"),
        Some(administrator.clone()),
        Some(json!({"name":format!("provider-updated-{suffix}"),"token":""})),
    )
    .await;
    assert_eq!(provider_patch.status(), StatusCode::OK);
    let provider = response_json(provider_patch).await;
    assert_eq!(provider["mountPath"], "secret");

    let external_secret_response = request(
        &app,
        Method::POST,
        "/api/v1/resourceBindings/secrets/external",
        Some(administrator.clone()),
        Some(json!({
            "name":format!("EXTERNAL_{suffix}").to_uppercase(),
            "providerId":provider_id,
            "externalPath":"services/citadel",
            "externalKey":"token",
            "externalVersion":3
        })),
    )
    .await;
    assert_eq!(external_secret_response.status(), StatusCode::OK);
    let external_secret = response_json(external_secret_response).await;
    let external_secret_id = external_secret["id"].as_str().unwrap();
    let external_secret_patch = request(
        &app,
        Method::PATCH,
        &format!("/api/v1/resourceBindings/secrets/external/{external_secret_id}"),
        Some(administrator.clone()),
        Some(json!({"externalKey":"rotated-token"})),
    )
    .await;
    assert_eq!(external_secret_patch.status(), StatusCode::OK);
    let external_secret = response_json(external_secret_patch).await;
    assert_eq!(external_secret["externalKey"], "rotated-token");
    assert_eq!(external_secret["externalVersion"], 3);

    let secret_binding_response = request(
        &app,
        Method::POST,
        "/api/v1/resourceBindings/global",
        Some(administrator.clone()),
        Some(json!({
            "name":"EXTERNAL_TOKEN",
            "kind":"Secret",
            "secretId":external_secret_id,
            "secretDeliveryMode":"EnvironmentVariable"
        })),
    )
    .await;
    assert_eq!(secret_binding_response.status(), StatusCode::OK);
    let secret_bindings = response_json(secret_binding_response).await;
    let secret_binding_id = secret_bindings["entries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["secretId"] == external_secret_id)
        .and_then(|entry| entry["id"].as_str())
        .unwrap()
        .to_owned();

    let secret_list = response_json(
        request(
            &app,
            Method::GET,
            "/api/v1/resourceBindings/secrets",
            Some(administrator.clone()),
            None,
        )
        .await,
    )
    .await;
    assert!(
        secret_list["secrets"]
            .as_array()
            .unwrap()
            .iter()
            .any(|secret| secret["id"] == external_secret_id)
    );

    assert_eq!(
        request(
            &app,
            Method::DELETE,
            &format!("/api/v1/resourceBindings/global/{secret_binding_id}"),
            Some(administrator.clone()),
            None,
        )
        .await
        .status(),
        StatusCode::OK
    );

    assert_eq!(
        request(
            &app,
            Method::DELETE,
            &format!("/api/v1/resourceBindings/secrets/{external_secret_id}"),
            Some(administrator.clone()),
            None,
        )
        .await
        .status(),
        StatusCode::NOT_FOUND,
        "deleting the last binding must delete its orphaned Secret definition"
    );
    assert_eq!(
        request(
            &app,
            Method::DELETE,
            &format!("/api/v1/resourceBindings/secrets/{internal_secret_id}"),
            Some(administrator.clone()),
            None,
        )
        .await
        .status(),
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        request(
            &app,
            Method::DELETE,
            "/api/v1/gitAccounts",
            Some(administrator.clone()),
            Some(json!({"ids":[git_account_id,basic_git_account_id,ssh_git_account_id]})),
        )
        .await
        .status(),
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        request(
            &app,
            Method::DELETE,
            &format!("/api/v1/resourceBindings/secret-providers/{provider_id}"),
            Some(administrator.clone()),
            None,
        )
        .await
        .status(),
        StatusCode::NO_CONTENT
    );

    assert_eq!(
        request(
            &app,
            Method::DELETE,
            "/api/v1/registries",
            Some(resource_actor.clone()),
            Some(json!({"ids":[registry_id]})),
        )
        .await
        .status(),
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        request(
            &app,
            Method::DELETE,
            "/api/v1/gitRepositories",
            Some(resource_actor),
            Some(json!({"ids":[repository_id]})),
        )
        .await
        .status(),
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        request(
            &app,
            Method::DELETE,
            &format!("/api/v1/tags/{tag_id}"),
            Some(administrator),
            None,
        )
        .await
        .status(),
        StatusCode::NO_CONTENT
    );
    let _ = tokio::fs::remove_dir_all(git_cache).await;
}

async fn request_raw(
    app: &Router,
    method: Method,
    uri: &str,
    principal: Option<ActorPrincipal>,
    body: Vec<u8>,
) -> axum::response::Response {
    let mut request = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(body))
        .unwrap();
    if let Some(principal) = principal {
        request.extensions_mut().insert(principal);
    }
    app.clone().oneshot(request).await.unwrap()
}

async fn request(
    app: &Router,
    method: Method,
    uri: &str,
    principal: Option<ActorPrincipal>,
    body: Option<Value>,
) -> axum::response::Response {
    let mut request = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(body.map_or_else(Vec::new, |value| {
            serde_json::to_vec(&value).unwrap()
        })))
        .unwrap();
    if let Some(principal) = principal {
        request.extensions_mut().insert(principal);
    }
    app.clone().oneshot(request).await.unwrap()
}

async fn request_with_header(
    app: &Router,
    method: Method,
    uri: &str,
    header: &str,
    value: &str,
    body: Option<Value>,
) -> axum::response::Response {
    app.clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(uri)
                .header("content-type", "application/json")
                .header(header, value)
                .body(Body::from(body.map_or_else(Vec::new, |value| {
                    serde_json::to_vec(&value).unwrap()
                })))
                .unwrap(),
        )
        .await
        .unwrap()
}

async fn response_json(response: axum::response::Response) -> Value {
    serde_json::from_slice(&to_bytes(response.into_body(), 1024 * 1024).await.unwrap()).unwrap()
}
