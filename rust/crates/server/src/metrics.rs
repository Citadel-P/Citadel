use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};

use sqlx::PgPool;

#[derive(Default)]
pub struct Metrics {
    active_tasks: AtomicI64,
    event_queue_depth: AtomicI64,
    docker_events_total: AtomicU64,
    docker_stream_reconnects_total: AtomicU64,
    readiness_failures_total: AtomicU64,
    event_lag_milliseconds: AtomicU64,
    agent_stats_samples_total: AtomicU64,
    agent_stream_reconnects_total: AtomicU64,
    agent_handshake_failures_total: AtomicU64,
    local_stats_samples_total: AtomicU64,
    realtime_active_connections: AtomicI64,
    realtime_events_published_total: AtomicU64,
    realtime_messages_sent_total: AtomicU64,
    realtime_overflows_total: AtomicU64,
    realtime_snapshot_resyncs_total: AtomicU64,
    realtime_authorization_failures_total: AtomicU64,
    realtime_send_timeouts_total: AtomicU64,
}

impl Metrics {
    pub fn task_guard(&self) -> TaskGuard<'_> {
        self.active_tasks.fetch_add(1, Ordering::Relaxed);
        TaskGuard { metrics: self }
    }

    pub fn event_enqueued(&self) {
        self.event_queue_depth.fetch_add(1, Ordering::Relaxed);
    }

    pub fn event_consumed(&self, lag_milliseconds: u64) {
        self.event_queue_depth.fetch_sub(1, Ordering::Relaxed);
        self.docker_events_total.fetch_add(1, Ordering::Relaxed);
        self.event_lag_milliseconds
            .store(lag_milliseconds, Ordering::Relaxed);
    }

    pub fn stream_reconnected(&self) {
        self.docker_stream_reconnects_total
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn readiness_failed(&self) {
        self.readiness_failures_total
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn agent_stats_sampled(&self) {
        self.agent_stats_samples_total
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn agent_stream_reconnected(&self) {
        self.agent_stream_reconnects_total
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn agent_handshake_failed(&self) {
        self.agent_handshake_failures_total
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn local_stats_sampled(&self) {
        self.local_stats_samples_total
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn realtime_connection_guard(&self) -> RealtimeConnectionGuard<'_> {
        self.realtime_active_connections
            .fetch_add(1, Ordering::Relaxed);
        RealtimeConnectionGuard { metrics: self }
    }

    pub fn realtime_event_published(&self) {
        self.realtime_events_published_total
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn realtime_message_sent(&self) {
        self.realtime_messages_sent_total
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn realtime_overflowed(&self) {
        self.realtime_overflows_total
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn realtime_snapshot_resynced(&self) {
        self.realtime_snapshot_resyncs_total
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn realtime_authorization_failed(&self) {
        self.realtime_authorization_failures_total
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn realtime_send_timed_out(&self) {
        self.realtime_send_timeouts_total
            .fetch_add(1, Ordering::Relaxed);
    }

    #[must_use]
    pub fn realtime_active_connections(&self) -> i64 {
        self.realtime_active_connections.load(Ordering::Relaxed)
    }

    #[must_use]
    pub fn render(&self, pool: &PgPool) -> String {
        format!(
            concat!(
                "# TYPE citadel_active_tasks gauge\n",
                "citadel_active_tasks {}\n",
                "# TYPE citadel_event_queue_depth gauge\n",
                "citadel_event_queue_depth {}\n",
                "# TYPE citadel_docker_events_total counter\n",
                "citadel_docker_events_total {}\n",
                "# TYPE citadel_docker_stream_reconnects_total counter\n",
                "citadel_docker_stream_reconnects_total {}\n",
                "# TYPE citadel_readiness_failures_total counter\n",
                "citadel_readiness_failures_total {}\n",
                "# TYPE citadel_event_lag_milliseconds gauge\n",
                "citadel_event_lag_milliseconds {}\n",
                "# TYPE citadel_agent_stats_samples_total counter\n",
                "citadel_agent_stats_samples_total {}\n",
                "# TYPE citadel_agent_stream_reconnects_total counter\n",
                "citadel_agent_stream_reconnects_total {}\n",
                "# TYPE citadel_agent_handshake_failures_total counter\n",
                "citadel_agent_handshake_failures_total {}\n",
                "# TYPE citadel_local_stats_samples_total counter\n",
                "citadel_local_stats_samples_total {}\n",
                "# TYPE citadel_realtime_active_connections gauge\n",
                "citadel_realtime_active_connections {}\n",
                "# TYPE citadel_realtime_events_published_total counter\n",
                "citadel_realtime_events_published_total {}\n",
                "# TYPE citadel_realtime_messages_sent_total counter\n",
                "citadel_realtime_messages_sent_total {}\n",
                "# TYPE citadel_realtime_overflows_total counter\n",
                "citadel_realtime_overflows_total {}\n",
                "# TYPE citadel_realtime_snapshot_resyncs_total counter\n",
                "citadel_realtime_snapshot_resyncs_total {}\n",
                "# TYPE citadel_realtime_authorization_failures_total counter\n",
                "citadel_realtime_authorization_failures_total {}\n",
                "# TYPE citadel_realtime_send_timeouts_total counter\n",
                "citadel_realtime_send_timeouts_total {}\n",
                "# TYPE citadel_postgres_pool_size gauge\n",
                "citadel_postgres_pool_size {}\n",
                "# TYPE citadel_postgres_pool_idle gauge\n",
                "citadel_postgres_pool_idle {}\n",
                "# EOF\n",
            ),
            self.active_tasks.load(Ordering::Relaxed),
            self.event_queue_depth.load(Ordering::Relaxed),
            self.docker_events_total.load(Ordering::Relaxed),
            self.docker_stream_reconnects_total.load(Ordering::Relaxed),
            self.readiness_failures_total.load(Ordering::Relaxed),
            self.event_lag_milliseconds.load(Ordering::Relaxed),
            self.agent_stats_samples_total.load(Ordering::Relaxed),
            self.agent_stream_reconnects_total.load(Ordering::Relaxed),
            self.agent_handshake_failures_total.load(Ordering::Relaxed),
            self.local_stats_samples_total.load(Ordering::Relaxed),
            self.realtime_active_connections.load(Ordering::Relaxed),
            self.realtime_events_published_total.load(Ordering::Relaxed),
            self.realtime_messages_sent_total.load(Ordering::Relaxed),
            self.realtime_overflows_total.load(Ordering::Relaxed),
            self.realtime_snapshot_resyncs_total.load(Ordering::Relaxed),
            self.realtime_authorization_failures_total
                .load(Ordering::Relaxed),
            self.realtime_send_timeouts_total.load(Ordering::Relaxed),
            pool.size(),
            pool.num_idle(),
        )
    }
}

pub struct RealtimeConnectionGuard<'a> {
    metrics: &'a Metrics,
}

impl Drop for RealtimeConnectionGuard<'_> {
    fn drop(&mut self) {
        self.metrics
            .realtime_active_connections
            .fetch_sub(1, Ordering::Relaxed);
    }
}

pub struct TaskGuard<'a> {
    metrics: &'a Metrics,
}

impl Drop for TaskGuard<'_> {
    fn drop(&mut self) {
        self.metrics.active_tasks.fetch_sub(1, Ordering::Relaxed);
    }
}
