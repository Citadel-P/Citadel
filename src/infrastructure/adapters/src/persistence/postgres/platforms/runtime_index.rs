//! Committed runtime identity hints. No resource details, health or ACLs are cached.
//! Every consuming SQL statement still verifies the complete runtime identity.
use sqlx::{PgPool, Postgres, Transaction};
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex, OnceLock, RwLock, Weak},
};
use uuid::Uuid;

type Scope = (Uuid, Option<String>);
type Identities = BTreeMap<String, Uuid>;
type Snapshot = BTreeMap<Scope, Arc<Identities>>;
pub(super) type Database = (String, u16, String, String, Option<std::path::PathBuf>);
static INDEXES: OnceLock<Mutex<BTreeMap<Database, Weak<RuntimeIdentityIndex>>>> = OnceLock::new();

#[derive(Default)]
struct State {
    revision: u64,
    initialized: bool,
    snapshot: Arc<Snapshot>,
}

pub struct RuntimeIdentityIndex {
    pool: PgPool,
    state: RwLock<State>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PrefixMatch {
    Unique(Uuid),
    Ambiguous,
}
pub(super) fn database(pool: &PgPool) -> Database {
    let options = pool.connect_options();
    (
        options.get_host().into(),
        options.get_port(),
        options
            .get_database()
            .unwrap_or(options.get_username())
            .into(),
        options.get_username().into(),
        options.get_socket().cloned(),
    )
}
impl RuntimeIdentityIndex {
    /// A process owner retains the index; the registry only holds weak references.
    pub fn attach(pool: PgPool) -> Arc<Self> {
        let mut indexes = INDEXES
            .get_or_init(Default::default)
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let key = database(&pool);
        if let Some(index) = indexes.get(&key).and_then(Weak::upgrade) {
            return index;
        }
        indexes.retain(|_, index| index.strong_count() > 0);
        let index = Arc::new(Self {
            pool,
            state: Default::default(),
        });
        indexes.insert(key, Arc::downgrade(&index));
        index
    }
    pub fn lookup(&self, platform: Uuid, node: Option<&str>, docker: &str) -> Option<Uuid> {
        self.state
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .snapshot
            .get(&(platform, node.map(str::to_owned)))
            .and_then(|scope| scope.get(docker))
            .copied()
    }
    /// Resolve a short Docker ID only from a complete committed snapshot. A
    /// miss remains eligible for SQL fallback; multiple scopes fail closed.
    pub fn lookup_prefix(&self, prefix: &str) -> Option<PrefixMatch> {
        let prefix = prefix.to_ascii_lowercase();
        let state = self.state.read().unwrap_or_else(|e| e.into_inner());
        if !state.initialized || prefix.is_empty() {
            return None;
        }
        let mut found = None;
        for identities in state.snapshot.values() {
            for (docker, id) in identities.range(prefix.clone()..) {
                if !docker.starts_with(&prefix) {
                    break;
                }
                if found.is_some_and(|found| found != *id) {
                    return Some(PrefixMatch::Ambiguous);
                }
                found = Some(*id);
            }
        }
        found.map(PrefixMatch::Unique)
    }
    pub fn initialized(&self) -> bool {
        self.state
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .initialized
    }
    /// Rebuild from committed PostgreSQL rows on startup. A concurrent publication
    /// invalidates this read, so an old rebuild cannot resurrect a removed identity.
    pub async fn rebuild(&self) -> Result<bool, sqlx::Error> {
        let revision = self
            .state
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .revision;
        let rows: Vec<(Uuid, Option<String>, String, Uuid)> =
            sqlx::query_as("SELECT platformid,dockernodeid,dockercontainerid,id FROM containers")
                .fetch_all(&self.pool)
                .await?;
        let mut snapshot = Snapshot::new();
        for (platform, node, docker, id) in rows {
            Arc::make_mut(snapshot.entry((platform, node)).or_default()).insert(docker, id);
        }
        Ok(self.install_rebuild(revision, snapshot))
    }
    fn install_rebuild(&self, revision: u64, snapshot: Snapshot) -> bool {
        let mut state = self.state.write().unwrap_or_else(|e| e.into_inner());
        if state.revision != revision {
            return false;
        }
        state.snapshot = Arc::new(snapshot);
        state.initialized = true;
        state.revision += 1;
        true
    }
    fn replace(&self, scope: Scope, identities: Identities) {
        let mut state = self.state.write().unwrap_or_else(|e| e.into_inner());
        let snapshot = Arc::make_mut(&mut state.snapshot);
        if identities.is_empty() {
            snapshot.remove(&scope);
        } else {
            snapshot.insert(scope, Arc::new(identities));
        }
        state.revision += 1;
    }
    fn observe(&self, scope: Scope, docker: &str, id: Option<Uuid>) {
        let mut state = self.state.write().unwrap_or_else(|e| e.into_inner());
        let snapshot = Arc::make_mut(&mut state.snapshot);
        if let Some(id) = id {
            Arc::make_mut(snapshot.entry(scope).or_default()).insert(docker.into(), id);
        } else if let Some(identities) = snapshot.get_mut(&scope) {
            Arc::make_mut(identities).remove(docker);
            if identities.is_empty() {
                snapshot.remove(&scope);
            }
        }
        state.revision += 1;
    }
}
pub(crate) fn attached(pool: &PgPool) -> Option<Arc<RuntimeIdentityIndex>> {
    INDEXES
        .get()?
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get(&database(pool))
        .and_then(Weak::upgrade)
}
pub(crate) fn hint(
    pool: &PgPool,
    platform: Uuid,
    node: Option<&str>,
    docker: &str,
) -> Option<Uuid> {
    use citadel_runtime::runtime_metrics::RuntimeWork;
    let _lookup = RuntimeWork::RuntimeIdentityLookup.start();
    let result = attached(pool).and_then(|index| index.lookup(platform, node, docker));
    RuntimeWork::RuntimeIdentityLookup.units(u64::from(result.is_some()));
    result
}
pub(crate) fn forget_platform(pool: &PgPool, platform: Uuid) {
    if let Some(index) = attached(pool) {
        let mut state = index.state.write().unwrap_or_else(|e| e.into_inner());
        Arc::make_mut(&mut state.snapshot).retain(|(id, _), _| *id != platform);
        state.revision += 1;
    }
}
/// Call synchronously after commit, while holding the projection write guard.
pub(crate) fn committed_identity(
    pool: &PgPool,
    platform: Uuid,
    node: Option<&str>,
    docker: &str,
    id: Option<Uuid>,
) {
    if let Some(index) = attached(pool) {
        index.observe((platform, node.map(str::to_owned)), docker, id);
    }
}

#[must_use = "publish only after the transaction commits; dropping rolls back the index update"]
pub(crate) struct PendingScope {
    index: Arc<RuntimeIdentityIndex>,
    scope: Scope,
    identities: Identities,
}
impl PendingScope {
    pub(crate) fn committed(self) {
        self.index.replace(self.scope, self.identities);
    }
}
/// Infrequent complete-set/deployment writes stage IDs under the same transaction
/// and resource guard. Targeted events publish their existing RETURNING/read IDs.
pub(crate) async fn stage_scope(
    pool: &PgPool,
    tx: &mut Transaction<'_, Postgres>,
    platform: Uuid,
    node: Option<&str>,
) -> Result<Option<PendingScope>, sqlx::Error> {
    let Some(index) = attached(pool) else {
        return Ok(None);
    };
    let rows: Vec<(String,Uuid)> = sqlx::query_as("SELECT dockercontainerid,id FROM containers WHERE platformid=$1 AND dockernodeid IS NOT DISTINCT FROM $2")
        .bind(platform).bind(node).fetch_all(&mut **tx).await?;
    Ok(Some(PendingScope {
        index,
        scope: (platform, node.map(str::to_owned)),
        identities: rows.into_iter().collect(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn index() -> RuntimeIdentityIndex {
        RuntimeIdentityIndex {
            pool: sqlx::postgres::PgPoolOptions::new()
                .connect_lazy("postgres://localhost/index_test")
                .unwrap(),
            state: Default::default(),
        }
    }
    #[tokio::test]
    async fn identities_are_scoped_and_old_snapshots_remain_immutable() {
        let index = index();
        let platform = Uuid::now_v7();
        let other = Uuid::now_v7();
        let local = Uuid::now_v7();
        let worker = Uuid::now_v7();
        index.observe((platform, None), "same", Some(local));
        let old = index.state.read().unwrap().snapshot.clone();
        index.observe((platform, Some("node".into())), "same", Some(worker));
        index.observe((other, None), "same", Some(worker));
        index.observe((platform, None), "same", None);
        assert_eq!(index.lookup(platform, None, "same"), None);
        assert_eq!(index.lookup(platform, Some("node"), "same"), Some(worker));
        assert_eq!(index.lookup(other, None, "same"), Some(worker));
        assert_eq!(old[&(platform, None)]["same"], local);
    }
    #[tokio::test]
    async fn stale_rebuild_cannot_restore_a_committed_deletion() {
        let index = index();
        let platform = Uuid::now_v7();
        let id = Uuid::now_v7();
        let stale = BTreeMap::from([(
            (platform, None),
            Arc::new(BTreeMap::from([("old".into(), id)])),
        )]);
        index.observe((platform, None), "old", None);
        assert!(!index.install_rebuild(0, stale));
        assert_eq!(index.lookup(platform, None, "old"), None);
    }
    #[tokio::test]
    async fn prefix_lookup_requires_a_complete_snapshot_and_fails_closed_on_ambiguity() {
        let index = index();
        assert_eq!(index.lookup_prefix("abc"), None);
        let platform = Uuid::now_v7();
        let first = Uuid::now_v7();
        let second = Uuid::now_v7();
        index.observe((platform, None), "abcdef01", Some(first));
        index.state.write().unwrap().initialized = true;
        assert_eq!(index.lookup_prefix("ABC"), Some(PrefixMatch::Unique(first)));
        index.observe((platform, Some("worker".into())), "abcdef02", Some(second));
        assert_eq!(index.lookup_prefix("abc"), Some(PrefixMatch::Ambiguous));
        assert_eq!(index.lookup_prefix("def"), None);
    }
}
