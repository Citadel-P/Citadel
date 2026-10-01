//! Process-local committed authorization state. All application ACL writers enter
//! `Mutation` before opening their transaction. Readers hold the gate through SQL
//! and cache publication, so a revoked grant cannot be resurrected by a cold read.
mod mutation;
mod reads;
#[cfg(test)]
mod tests;
pub(crate) use mutation::{Impact, Mutation};

use citadel_identity::{AuthorizationSnapshot, PermissionGrant};
use sqlx::PgPool;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, Mutex, OnceLock, Weak},
    time::{Duration, Instant},
};
use tokio::sync::OwnedRwLockReadGuard;
use tokio::sync::{RwLock, watch};
use uuid::Uuid;

const ACTOR_CAPACITY: usize = 1024;
const RESOURCE_CAPACITY: usize = 8192;
const SAFETY_TTL: Duration = Duration::from_secs(300);
type Database = (String, u16, String, String, Option<std::path::PathBuf>);
static CACHES: OnceLock<Mutex<BTreeMap<Database, Weak<AuthorizationCache>>>> = OnceLock::new();
type ResourceKey = (Uuid, i32, Uuid);

pub(crate) struct AuthorizationCache {
    gate: Arc<RwLock<()>>,
    state: Mutex<State>,
}
/// Hold across the transaction that consumes a cached authorization decision.
/// ACL mutations take the matching write gate through commit and invalidation.
pub(crate) struct ReadFence {
    _guard: OwnedRwLockReadGuard<()>,
}
#[derive(Default)]
struct State {
    clock: u64,
    actors: BTreeMap<Uuid, ActorEntry>,
    actor_order: BTreeSet<(u64, Uuid)>,
    resources: BTreeMap<ResourceKey, ResourceEntry>,
    resource_order: BTreeSet<(u64, ResourceKey)>,
}
struct ActorEntry {
    // A fresh UUID on eviction/reload prevents an old resource entry matching a
    // newly admitted actor. The same generation drives realtime revocation.
    generation: watch::Sender<Uuid>,
    used: u64,
    loaded: Option<Arc<ActorScope>>,
}
struct ActorScope {
    actors: Vec<Uuid>,
    snapshot: AuthorizationSnapshot,
    expires: Instant,
}
struct ResourceEntry {
    generation: Uuid,
    grant: Option<PermissionGrant>,
    expires: Instant,
    used: u64,
}
impl AuthorizationCache {
    pub(crate) fn attach(pool: &PgPool) -> Arc<Self> {
        let options = pool.connect_options();
        let key = (
            options.get_host().into(),
            options.get_port(),
            options
                .get_database()
                .unwrap_or(options.get_username())
                .into(),
            options.get_username().into(),
            options.get_socket().cloned(),
        );
        let mut caches = CACHES
            .get_or_init(Default::default)
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        if let Some(cache) = caches.get(&key).and_then(Weak::upgrade) {
            return cache;
        }
        caches.retain(|_, cache| cache.strong_count() > 0);
        let cache = Arc::new(Self {
            gate: Arc::new(RwLock::new(())),
            state: Mutex::new(State::default()),
        });
        caches.insert(key, Arc::downgrade(&cache));
        cache
    }
    pub(crate) fn subscribe(&self, actor: Uuid) -> watch::Receiver<Uuid> {
        self.state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .actor(actor)
            .generation
            .subscribe()
    }
    pub(crate) async fn read_fence(&self) -> ReadFence {
        ReadFence {
            _guard: self.gate.clone().read_owned().await,
        }
    }
    fn invalidate(&self, actors: impl IntoIterator<Item = Uuid>) {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        for actor in actors {
            if let Some(entry) = state.actors.get_mut(&actor) {
                entry.loaded = None;
                citadel_runtime::runtime_metrics::RuntimeWork::AuthorizationInvalidation.units(1);
                entry.generation.send_replace(Uuid::now_v7());
            }
        }
    }
}
impl State {
    fn actor(&mut self, id: Uuid) -> &mut ActorEntry {
        self.clock = self.clock.wrapping_add(1);
        if !self.actors.contains_key(&id)
            && self.actors.len() >= ACTOR_CAPACITY
            && let Some((_, oldest)) = self.actor_order.pop_first()
        {
            // Closing the watch fails subscriptions closed. Eviction drops
            // generation metadata as well as the cached scope/snapshot.
            self.actors.remove(&oldest);
        }
        let entry = self.actors.entry(id).or_insert_with(|| ActorEntry {
            generation: watch::channel(Uuid::now_v7()).0,
            used: 0,
            loaded: None,
        });
        self.actor_order.remove(&(entry.used, id));
        entry.used = self.clock;
        self.actor_order.insert((entry.used, id));
        entry
    }
    fn resource(&mut self, key: ResourceKey, generation: Uuid) -> Option<Option<PermissionGrant>> {
        let entry = self.resources.get_mut(&key)?;
        if entry.generation != generation || entry.expires <= Instant::now() {
            return None;
        }
        self.clock = self.clock.wrapping_add(1);
        self.resource_order.remove(&(entry.used, key));
        entry.used = self.clock;
        self.resource_order.insert((entry.used, key));
        Some(entry.grant)
    }
    fn put_resource(
        &mut self,
        key: ResourceKey,
        generation: Uuid,
        grant: Option<PermissionGrant>,
        scope_expires: Instant,
    ) {
        self.clock = self.clock.wrapping_add(1);
        if !self.resources.contains_key(&key)
            && self.resources.len() >= RESOURCE_CAPACITY
            && let Some((_, oldest)) = self.resource_order.pop_first()
        {
            self.resources.remove(&oldest);
        }
        if let Some(old) = self.resources.insert(
            key,
            ResourceEntry {
                generation,
                grant,
                expires: scope_expires.min(Instant::now() + SAFETY_TTL),
                used: self.clock,
            },
        ) {
            self.resource_order.remove(&(old.used, key));
        }
        self.resource_order.insert((self.clock, key));
    }
}
