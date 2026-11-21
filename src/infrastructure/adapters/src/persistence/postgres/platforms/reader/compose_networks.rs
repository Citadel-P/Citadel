//! Parsed Compose references may be shared across readers. SQL still authorizes
//! each owner and validates its release revision in the same statement.
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use uuid::Uuid;

#[derive(Default)]
pub(super) struct ComposeNetworks(Mutex<HashMap<Uuid, Arc<Document>>>);

pub(super) struct Document {
    pub platform: Uuid,
    pub revision: String,
    pub bytes: i32,
    pub project: Option<String>,
    pub names: Vec<String>,
    inserted: Instant,
    cost: usize,
}

impl ComposeNetworks {
    pub fn snapshot(&self, platform: Uuid) -> HashMap<Uuid, Arc<Document>> {
        self.0
            .lock()
            .unwrap()
            .iter()
            .filter(|(_, doc)| {
                doc.platform == platform && doc.inserted.elapsed() < Duration::from_secs(300)
            })
            .map(|(&id, doc)| (id, doc.clone()))
            .collect()
    }

    pub fn remember(
        &self,
        id: Uuid,
        platform: Uuid,
        revision: String,
        bytes: i32,
        project: Option<String>,
        names: Vec<String>,
    ) {
        const LIMIT: usize = 4 * 1024 * 1024;
        let cost = names.iter().map(|s| s.capacity()).sum::<usize>()
            + names.capacity() * size_of::<String>()
            + project.as_ref().map_or(0, String::capacity);
        if cost > LIMIT {
            return;
        }
        let mut cache = self.0.lock().unwrap();
        if cache.len() >= 512 || cache.values().map(|d| d.cost).sum::<usize>() + cost > LIMIT {
            cache.clear();
        }
        cache.insert(
            id,
            Arc::new(Document {
                platform,
                revision,
                bytes,
                project,
                names,
                cost,
                inserted: Instant::now(),
            }),
        );
    }
}
