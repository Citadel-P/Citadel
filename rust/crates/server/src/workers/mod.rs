mod alerts;
mod automation;
mod backups;
mod build_consumers;
mod builds;
mod containers;
mod deployments;
pub mod edge;
mod git;
mod platforms;
mod stacks;
mod swarm_services;
mod volume_helpers;

pub use platforms::{WorkerDependencies, WorkerSettings, register};

mod disk;

#[cfg(test)]
mod disk_tests;

mod maintenance;
mod notifications;
mod schedule;
mod unmanaged;

mod pruning;

// LISTEN holds its connection for the stream lifetime. Keep that bounded
// connection separate from the request/worker pool to avoid starving queries.
async fn listener(
    pool: &sqlx::PgPool,
    channel: &str,
) -> Result<sqlx::postgres::PgListener, sqlx::Error> {
    let dedicated = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect_lazy_with((*pool.connect_options()).clone().application_name(channel));
    let mut listener = sqlx::postgres::PgListener::connect_with(&dedicated).await?;
    listener.listen(channel).await?;
    Ok(listener)
}

pub mod image_scanning;

mod stats_alerts;

#[cfg(test)]
mod runtime_persistence_tests;
