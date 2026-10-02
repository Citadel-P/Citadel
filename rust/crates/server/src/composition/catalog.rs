//! Catalog service construction.
use citadel_activities::ActivityService;
use citadel_adapters::persistence::postgres::activities::store::PostgresActivityStore;
use citadel_adapters::persistence::postgres::bindings::PostgresBindingRepository;
use citadel_adapters::persistence::postgres::git::accounts::PostgresGitAccountRepository;
use citadel_adapters::persistence::postgres::git::repositories::PostgresGitRepositoryExecutionPersistence;
use citadel_adapters::security::identity::crypto::AesGcmSecretProtector;
use citadel_bindings::SecretService;
use citadel_git::{GitAccountService, GitCli, GitRepositoryExecutionService};
use citadel_server::config::Config;
use citadel_server::realtime::RealtimeHub;
use sqlx::PgPool;
use std::sync::Arc;
use std::time::Duration;
pub(super) struct CatalogComponents {
    pub activities: Arc<ActivityService>,
    pub tags: Arc<dyn citadel_tags::TagRepository>,
    pub registries: Arc<dyn citadel_registries::RegistryRepository>,
    pub registry_connections: Arc<dyn citadel_registries::RegistryConnectionChecker>,
    pub secrets: Arc<SecretService>,
    pub git_accounts: Arc<GitAccountService>,
    pub git_execution: Arc<GitRepositoryExecutionService>,
    pub search: Arc<citadel_adapters::persistence::postgres::discovery::search::PostgresGlobalSearchStore>,
    pub actors: Arc<citadel_adapters::persistence::postgres::identity::actors::repository::PostgresActorRepository>,
    pub lookup: Arc<citadel_adapters::persistence::postgres::discovery::lookup::PostgresLookupStore>,
    pub audit: Arc<PostgresActivityStore>,
}

pub(super) fn build(
    config: &Config,
    pool: &PgPool,
    secret_protector: &Arc<AesGcmSecretProtector>,
    realtime_hub: &Option<RealtimeHub>,
    tasks: &citadel_runtime::DynamicTasks,
) -> Result<CatalogComponents, Box<dyn std::error::Error>> {
    let activities = Arc::new(ActivityService::new(Arc::new(PostgresActivityStore::new(
        pool.clone(),
    ))));
    let tags: Arc<dyn citadel_tags::TagRepository> = Arc::new(
        citadel_adapters::persistence::postgres::tags::PostgresTagRepository::new(pool.clone()),
    );
    let registries: Arc<dyn citadel_registries::RegistryRepository> = Arc::new(
        citadel_adapters::persistence::postgres::registries::PostgresRegistryRepository::new(
            pool.clone(),
        ),
    );
    let binding_store = Arc::new(PostgresBindingRepository::new(pool.clone()));
    let secrets = Arc::new(
        SecretService::new(binding_store, secret_protector.clone())
            .with_secret_provider_tester(Arc::new(
                citadel_adapters::persistence::postgres::bindings::secret_resolver::PostgresSecretValueResolver::new(
                    pool.clone(),
                    secret_protector.clone(),
                )?,
            )),
    );
    let git_accounts = Arc::new(GitAccountService::new(
        Arc::new(PostgresGitAccountRepository::new(pool.clone())),
        secret_protector.clone(),
    ));
    let data_root = &config.execution.paths.data_root;
    let git_execution = Arc::new(
        GitRepositoryExecutionService::new(
            Arc::new(PostgresGitRepositoryExecutionPersistence::new(pool.clone())),
            Arc::clone(&git_accounts),
            Arc::new(
                GitCli::new(
                    std::sync::Arc::new(citadel_processes::SystemProcess),
                    std::sync::Arc::new(
                        citadel_adapters::filesystem::git_workspace::LocalGitWorkspace::new(
                            tasks.clone(),
                        ),
                    ),
                    Duration::from_secs(120),
                )
                .with_known_hosts(config.execution.git_known_hosts.clone()),
            ),
            data_root.join("git-repositories"),
            Duration::from_secs(10 * 60),
        )
        .with_change_notifier(citadel_server::realtime::change_callback(
            realtime_hub.clone(),
            "GitRepository",
        )),
    );
    let search = Arc::new(
        citadel_adapters::persistence::postgres::discovery::search::PostgresGlobalSearchStore::new(
            pool.clone(),
        ),
    );
    let actors = Arc::new(citadel_adapters::persistence::postgres::identity::actors::repository::PostgresActorRepository::new(
        pool.clone(),
    ));
    let lookup = Arc::new(
        citadel_adapters::persistence::postgres::discovery::lookup::PostgresLookupStore::new(
            pool.clone(),
        ),
    );
    let audit = Arc::new(PostgresActivityStore::new(pool.clone()));
    Ok(CatalogComponents {
        activities,
        tags,
        registries,
        registry_connections: Arc::new(
            citadel_adapters::connectors::registries::browser::RegistryBrowser::new()?,
        ),
        secrets,
        git_accounts,
        git_execution,
        search,
        actors,
        lookup,
        audit,
    })
}
