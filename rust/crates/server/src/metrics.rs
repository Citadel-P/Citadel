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
            pool.size(),
            pool.num_idle(),
        )
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
