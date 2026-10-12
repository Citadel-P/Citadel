use chrono::{DateTime, Duration, Utc};
use sha2::{Digest, Sha256};
use std::{collections::HashMap, sync::Mutex};
use uuid::Uuid;

const MAX_KEYS: usize = 10_000;
const MAX_ATTEMPTS: u32 = 5;

#[derive(Default)]
pub(super) struct LoginThrottle(Mutex<Windows>);
#[derive(Default)]
struct Windows {
    entries: HashMap<[u8; 32], Window>,
    last_pruned: Option<DateTime<Utc>>,
}
struct Window {
    until: DateTime<Utc>,
    attempts: u32,
}
impl LoginThrottle {
    // Account IDs unify name/email aliases. Unknown identifiers get the same
    // admission policy without retaining identifiers or passwords in memory.
    pub fn key(user: Option<Uuid>, identifier: &str) -> [u8; 32] {
        Sha256::digest(match user {
            Some(id) => format!("user:{id}"),
            None => format!("unknown:{}", identifier.trim().to_lowercase()),
        })
        .into()
    }
    pub fn admit(&self, key: [u8; 32], now: DateTime<Utc>) -> bool {
        let mut state = self.0.lock().expect("login throttle poisoned");
        if state.entries.len() >= MAX_KEYS && !state.entries.contains_key(&key) {
            // Existing accounts never need a sweep. At capacity, new identifiers
            // can trigger at most one expiry sweep per second, including denials.
            if state
                .last_pruned
                .is_none_or(|last| now >= last + Duration::seconds(1))
            {
                state.entries.retain(|_, entry| entry.until > now);
                state.last_pruned = Some(now);
            }
            if state.entries.len() >= MAX_KEYS {
                return false;
            }
        }
        let entries = &mut state.entries;
        let entry = entries.entry(key).or_insert(Window {
            until: now + Duration::seconds(60),
            attempts: 0,
        });
        if entry.until <= now {
            *entry = Window {
                until: now + Duration::seconds(60),
                attempts: 0,
            };
        }
        if entry.attempts >= MAX_ATTEMPTS {
            return false;
        }
        // Reserve before hashing so concurrent failed requests cannot bypass the limit.
        entry.attempts += 1;
        true
    }
    pub fn succeeded(&self, key: [u8; 32]) {
        self.0
            .lock()
            .expect("login throttle poisoned")
            .entries
            .remove(&key);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn aliases_share_the_limit_and_unrelated_accounts_remain_available() {
        let throttle = LoginThrottle::default();
        let id = Uuid::now_v7();
        let key = LoginThrottle::key(Some(id), "owner");
        assert_eq!(key, LoginThrottle::key(Some(id), "owner@example.test"));
        let now = Utc::now();
        for _ in 0..MAX_ATTEMPTS {
            assert!(throttle.admit(key, now));
        }
        assert!(!throttle.admit(key, now));
        assert!(throttle.admit(LoginThrottle::key(None, "other"), now));
        assert!(throttle.admit(key, now + Duration::seconds(60)));
        throttle.succeeded(key);
        assert!(throttle.admit(key, now + Duration::seconds(60)));
    }
    #[test]
    fn saturated_accounts_remain_usable_and_sweeps_are_spaced() {
        let throttle = LoginThrottle::default();
        let now = Utc::now();
        for i in 0..MAX_KEYS {
            assert!(throttle.admit(LoginThrottle::key(None, &i.to_string()), now));
        }
        let extra = LoginThrottle::key(None, "extra");
        assert!(!throttle.admit(extra, now));
        let later = now + Duration::milliseconds(500);
        assert!(!throttle.admit(extra, later));
        assert!(throttle.admit(LoginThrottle::key(None, "0"), later));
        assert_eq!(throttle.0.lock().unwrap().last_pruned, Some(now));
        // Releasing an account slot must admit a new key without waiting for a sweep.
        throttle.succeeded(LoginThrottle::key(None, "0"));
        assert!(throttle.admit(extra, later));
    }

    #[test]
    fn saturation_is_bounded_and_expired_entries_are_reclaimed() {
        let throttle = LoginThrottle::default();
        let now = Utc::now();
        for i in 0..MAX_KEYS {
            assert!(throttle.admit(LoginThrottle::key(None, &i.to_string()), now));
        }
        let extra = LoginThrottle::key(None, "extra");
        assert!(!throttle.admit(extra, now));
        assert!(throttle.admit(extra, now + Duration::seconds(60)));
        assert_eq!(throttle.0.lock().unwrap().entries.len(), 1);
    }
}
