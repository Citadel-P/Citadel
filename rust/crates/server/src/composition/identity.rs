//! Identity service construction.
use citadel_adapters::connectors::oidc::protocol::OidcHttpProtocol;
use citadel_adapters::persistence::postgres::identity::authentication::store::PostgresIdentityStore;
use citadel_adapters::persistence::postgres::identity::mfa::store::PostgresMfaStore;
use citadel_adapters::persistence::postgres::identity::oidc::store::PostgresOidcStore;
use citadel_adapters::persistence::postgres::identity::profile::repository::PostgresProfileRepository;
use citadel_adapters::persistence::postgres::identity::roles::repository::PostgresRoleRepository;
use citadel_adapters::persistence::postgres::identity::service_accounts::repository::PostgresServiceAccountRepository;
use citadel_adapters::persistence::postgres::identity::service_accounts::usage::service_account_last_used_channel;
use citadel_adapters::persistence::postgres::identity::teams::repository::PostgresTeamRepository;
use citadel_adapters::persistence::postgres::identity::users::repository::PostgresUserRepository;
use citadel_adapters::persistence::postgres::licensing::store::PostgresLicenseEntitlementService;
use citadel_adapters::security::identity::crypto::AesGcmSecretProtector;
use citadel_adapters::security::identity::crypto::Argon2PasswordHasher;
use citadel_adapters::security::identity::crypto::JwtSessionTokenCodec;
use citadel_adapters::security::identity::crypto::OpaqueServiceAccountTokenCodec;
use citadel_adapters::security::identity::mfa::HmacRecoveryCodeService;
use citadel_adapters::security::identity::mfa::Sha1TotpService;
use citadel_identity::{
    IdentityService, MfaConfiguration, MfaService, OidcService, ProfileService,
    RoleMutationService, RoleReadService, ServiceAccountService, SystemClock, TeamMutationService,
    TeamReadService, UserReadService,
};
use citadel_server::config::Config;
use sqlx::PgPool;
use std::sync::Arc;
use std::time::Duration;
pub(super) struct IdentityComponents {
    pub identity: Arc<IdentityService>,
    pub mfa: Arc<MfaService>,
    pub oidc: Arc<OidcService>,
    pub service_accounts: Arc<ServiceAccountService>,
    pub profiles: Arc<ProfileService>,
    pub users: Arc<UserReadService>,
    pub user_mutations: Arc<citadel_identity::UserMutationService>,
    pub teams: Arc<TeamReadService>,
    pub team_mutations: Arc<TeamMutationService>,
    pub roles: Arc<RoleReadService>,
    pub role_mutations: Arc<RoleMutationService>,
    pub secret_protector: Arc<AesGcmSecretProtector>,
    pub last_used_worker: citadel_adapters::persistence::postgres::identity::service_accounts::usage::ServiceAccountLastUsedWorker,
}

pub(super) fn build(
    config: &Config,
    pool: &PgPool,
    entitlements: &Arc<PostgresLicenseEntitlementService>,
) -> Result<IdentityComponents, Box<dyn std::error::Error>> {
    let token_codec = Arc::new(JwtSessionTokenCodec::new(
        config.identity.jwt_key.expose(),
        config.identity.issuer.clone(),
        config.identity.audience.clone(),
    )?);
    let service_account_tokens = Arc::new(OpaqueServiceAccountTokenCodec);
    let clock = Arc::new(SystemClock);
    let password_hasher = Arc::new(Argon2PasswordHasher::default());
    let identity_store = Arc::new(PostgresIdentityStore::new(pool.clone()));
    let (last_used_tracker, last_used_worker) = service_account_last_used_channel(
        identity_store.clone(),
        config.identity.service_account_last_used_capacity,
        Duration::from_secs(30),
        config.identity.service_account_last_used_interval,
    );
    let identity = Arc::new(
        IdentityService::new(
            identity_store.clone(),
            password_hasher.clone(),
            token_codec,
            service_account_tokens.clone(),
            entitlements.clone(),
            clock.clone(),
            Arc::new(last_used_tracker),
            chrono::Duration::from_std(config.identity.access_token_lifetime)?,
            chrono::Duration::from_std(config.identity.refresh_token_lifetime)?,
        )
        .with_password_policy(config.identity.password_policy),
    );
    let secret_protector = Arc::new(AesGcmSecretProtector::new(
        config.identity.secret_encryption_key.expose(),
    )?);
    let mfa = Arc::new(MfaService::new(
        Arc::new(PostgresMfaStore::new(pool.clone())),
        Arc::clone(&identity),
        Arc::new(Sha1TotpService),
        secret_protector.clone(),
        Arc::new(HmacRecoveryCodeService::new(
            config.identity.secret_encryption_key.expose(),
        )?),
        clock.clone(),
        MfaConfiguration {
            policy: config.identity.mfa.policy,
            challenge_lifetime: chrono::Duration::from_std(config.identity.mfa.challenge_lifetime)?,
            setup_lifetime: chrono::Duration::from_std(config.identity.mfa.setup_lifetime)?,
            maximum_failed_attempts: config.identity.mfa.maximum_failed_attempts,
            recovery_code_count: config.identity.mfa.recovery_code_count,
        },
    ));
    let oidc = Arc::new(OidcService::new(
        Arc::new(PostgresOidcStore::new(pool.clone())),
        Arc::new(OidcHttpProtocol::new(Duration::from_secs(15))?),
        secret_protector.clone(),
        Arc::clone(&identity),
        clock.clone(),
        chrono::Duration::minutes(10),
    ));
    let service_accounts = Arc::new(
        ServiceAccountService::new(
            Arc::new(PostgresServiceAccountRepository::new(pool.clone())),
            service_account_tokens,
            entitlements.clone(),
            clock.clone(),
        )
        .with_limits(config.identity.service_account_limits)?,
    );
    let profiles = Arc::new(ProfileService::new(
        Arc::new(PostgresProfileRepository::new(pool.clone())),
        Arc::clone(&identity),
        clock.clone(),
    ));
    let user_store = Arc::new(PostgresUserRepository::new(pool.clone()));
    let users = Arc::new(UserReadService::new(user_store.clone()));
    let user_mutations = Arc::new(
        citadel_identity::UserMutationService::new(
            user_store,
            password_hasher,
            entitlements.clone(),
            clock.clone(),
        )
        .with_password_policy(config.identity.password_policy),
    );
    let team_store = Arc::new(PostgresTeamRepository::new(pool.clone()));
    let teams = Arc::new(TeamReadService::new(team_store.clone()));
    let team_mutations = Arc::new(TeamMutationService::new(
        team_store,
        entitlements.clone(),
        clock.clone(),
    ));
    let role_store = Arc::new(PostgresRoleRepository::new(pool.clone()));
    let roles = Arc::new(RoleReadService::new(role_store.clone()));
    let role_mutations = Arc::new(RoleMutationService::new(
        role_store,
        entitlements.clone(),
        clock,
    ));
    Ok(IdentityComponents {
        identity,
        mfa,
        oidc,
        service_accounts,
        profiles,
        users,
        user_mutations,
        teams,
        team_mutations,
        roles,
        role_mutations,
        secret_protector,
        last_used_worker,
    })
}
