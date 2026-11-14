/// Snapshot of the responding Core's query pool and connected PostgreSQL server.
/// Dedicated notification and coordination connections are outside this pool.
#[derive(Debug, Clone)]
pub struct DatabaseConnections {
    pub maximum: u32,
    pub open: u32,
    pub idle: u32,
    pub borrowed: u32,
    /// Unavailable when the server setting cannot be read within the request budget.
    pub shared_buffers_bytes: Option<u64>,
}
