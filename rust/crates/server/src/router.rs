//! Compose feature routes and middleware for the HTTP and edge gRPC listeners.
use crate::state::AppState;
use axum::{Router, middleware};
use citadel_server::api::deployments;
use citadel_server::api::stacks as stacks_http;
use citadel_server::api::swarm_services as swarm_services_http;
use citadel_server::config::Config;
use citadel_server::{
    activities_http, api::alerts, api::automation, application_info_http, identity_http,
    license_http, oidc_http, platforms_http, profile_http, resources_http, roles_http,
    service_accounts_http, teams_http, transport, users_http, webhooks_http,
};
use std::sync::Arc;
use tower_http::{catch_panic::CatchPanicLayer, trace::TraceLayer};

pub struct Routers {
    pub http: Router,
    pub edge: Router,
}

pub fn router(state: AppState, config: &Config) -> Result<Routers, Box<dyn std::error::Error>> {
    let AppState {
        pool,
        cancellation,
        readiness,
        metrics,
        realtime_hub,
        identity,
        licenses,
        entitlements,
        mfa,
        oidc,
        service_accounts,
        profiles,
        activities,
        resources,
        git_accounts,
        git_execution,
        agent_setup,
        agent_setup_context,
        edge_context,
        edge_registry,
        alert_store,
        alert_delivery,
        automation,
        backups,
        users,
        user_mutations,
        teams,
        team_mutations,
        roles,
        role_mutations,
        builds,
        deployments,
        swarm_services,
        stacks,
        platform_state,
        realtime,
        license_realtime_service,
        search,
        actors,
        lookup,
        audit,
        ..
    } = state;
    let app = citadel_server::diagnostics_http::router(
        citadel_server::diagnostics_http::DiagnosticsHttpState {
            readiness: Arc::clone(&readiness),
            metrics,
            pool: pool.clone(),
        },
    )
    .merge(identity_http::router(identity_http::IdentityHttpState {
        identity: Arc::clone(&identity),
        mfa,
        readiness: Arc::clone(&readiness),
        secure_cookies: config.transport.mode != citadel_server::config::TransportMode::Disabled,
    }))
    .merge(oidc_http::router(oidc_http::OidcHttpState {
        oidc,
        public_url: config.transport.public_url.clone(),
        allowed_return_origins: config.transport.cors_origins.clone(),
        secure_cookies: config.transport.mode != citadel_server::config::TransportMode::Disabled,
    }))
    .merge(application_info_http::router())
    .merge(citadel_server::search_http::router(search))
    .merge(citadel_server::actors_http::router(actors))
    .merge(license_http::router(license_http::LicenseHttpState {
        identity: Arc::clone(&identity),
        licenses,
    }))
    .merge(activities_http::router(
        activities_http::ActivitiesHttpState { activities },
    ))
    .merge(users_http::router(users_http::UsersHttpState {
        identity: Arc::clone(&identity),
        users,
        mutations: user_mutations,
    }))
    .merge(teams_http::router(teams_http::TeamsHttpState {
        identity: Arc::clone(&identity),
        teams,
        mutations: team_mutations,
    }))
    .merge(roles_http::router(roles_http::RolesHttpState {
        identity: Arc::clone(&identity),
        roles,
        mutations: role_mutations,
    }))
    .merge(service_accounts_http::router(
        service_accounts_http::ServiceAccountHttpState {
            identity: Arc::clone(&identity),
            service_accounts,
        },
    ))
    .merge(citadel_server::api::git::accounts::handlers::router(
        citadel_server::api::git::accounts::handlers::GitAccountsHttpState {
            identity: Arc::clone(&identity),
            accounts: git_accounts,
            realtime: realtime_hub.clone(),
        },
    ))
    .merge(citadel_server::api::git::repositories::handlers::router(
        citadel_server::api::git::repositories::handlers::GitRepositoriesHttpState {
            identity: Arc::clone(&identity),
            resources: Arc::clone(&resources),
            execution: Arc::clone(&git_execution),
            realtime: realtime_hub.clone(),
            cancellation: cancellation.clone(),
        },
    ))
    .merge(webhooks_http::router(webhooks_http::WebhooksHttpState {
        git: Arc::clone(&git_execution),
        automation: Arc::clone(&automation),
        backups: Some(backups.clone()),
        builds: Some(builds.clone()),
        stacks: Some(stacks.clone()),
        services: Some(swarm_services.clone()),
        audit: Some(audit),
        alerts: Some(alert_store.clone()),
    }))
    .merge(automation::router(automation::AutomationHttpState {
        identity: Arc::clone(&identity),
        automation,
    }))
    .merge(citadel_server::api::builds::handlers::router(
        citadel_server::api::builds::handlers::BuildsHttpState {
            identity: Arc::clone(&identity),
            builds,
        },
    ))
    .merge(citadel_server::api::backups::handlers::router(
        citadel_server::api::backups::handlers::BackupsHttpState {
            identity: Arc::clone(&identity),
            backups,
            cancellation: cancellation.clone(),
        },
    ))
    .merge(alerts::router(alerts::AlertsHttpState {
        identity: Arc::clone(&identity),
        store: alert_store,
        delivery: alert_delivery,
    }))
    .merge(deployments::router(deployments::DeploymentsHttpState {
        identity: Arc::clone(&identity),
        deployments: Arc::clone(&deployments),
    }))
    .merge(swarm_services_http::router(
        swarm_services_http::SwarmServicesHttpState {
            identity: Arc::clone(&identity),
            services: Arc::clone(&swarm_services),
        },
    ))
    .merge(stacks_http::router(stacks_http::StacksHttpState {
        identity: Arc::clone(&identity),
        stacks,
    }))
    .merge(resources_http::router(resources_http::ResourcesHttpState {
        identity: Arc::clone(&identity),
        resources,
        realtime: realtime_hub.clone(),
    }))
    .merge({
        platforms_http::router(platform_state.clone())
            .layer(axum::Extension(agent_setup))
            .layer(axum::Extension(agent_setup_context))
            .merge(citadel_server::lookup_http::router(
                citadel_server::lookup_http::LookupHttpState {
                    store: lookup,
                    entitlements: entitlements.clone(),
                    platforms: platform_state,
                },
            ))
    })
    .merge(profile_http::router(profile_http::ProfileHttpState {
        profiles,
    }));
    let mut app = if let Some(realtime) = realtime {
        app.merge(realtime.router())
    } else if let Some(license_realtime) = license_realtime_service {
        app.merge(license_realtime.router())
    } else {
        app
    };
    if let Some(hub) = realtime_hub {
        app = app.layer(axum::Extension(hub));
    }
    if config.transport.openapi_enabled {
        app = app.merge(citadel_server::openapi::serving::router(
            if config.transport.mode == citadel_server::config::TransportMode::Direct {
                "https"
            } else {
                "http"
            },
        ));
    }
    let edge_service = citadel_contracts::citadel::edge::v1::edge_agent_service_server::EdgeAgentServiceServer::new(
        citadel_adapters::edge::EdgeIntake::new(citadel_adapters::edge::PostgresEdgeStore::new(pool.clone()), edge_registry.clone()),
    ).max_decoding_message_size(citadel_adapters::edge::MAX_PAYLOAD + 4096)
        .max_encoding_message_size(citadel_adapters::edge::MAX_PAYLOAD + 4096);
    let edge_routes = tonic::service::Routes::new(edge_service).into_axum_router();
    app = app.layer(axum::Extension(edge_context));
    let mut edge_transport = config.transport.clone();
    edge_transport.static_root = None;
    let edge_app = transport::secure_router_with_grpc(
        Router::new(),
        &edge_transport,
        Arc::clone(&readiness),
        Some(edge_routes),
    )?;
    let app = transport::secure_router(
        app.layer(middleware::from_fn_with_state(
            identity,
            identity_http::authentication_middleware,
        ))
        .layer(CatchPanicLayer::custom(transport::panic_response))
        .layer(TraceLayer::new_for_http()),
        &config.transport,
        Arc::clone(&readiness),
    )?;
    Ok(Routers {
        http: app,
        edge: edge_app,
    })
}
