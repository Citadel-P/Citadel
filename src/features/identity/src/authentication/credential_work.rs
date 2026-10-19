use super::{IdentityError, PasswordHasher};
use std::sync::Arc;
use tokio::sync::Semaphore;

/// One shared CPU budget for authentication and user password mutations.
pub struct CredentialWork {
    passwords: Arc<dyn PasswordHasher>,
    credential_workers: Arc<Semaphore>,
}

impl CredentialWork {
    pub(super) fn dummy_hash(&self) -> String {
        self.passwords.dummy_hash()
    }
    pub fn new(passwords: Arc<dyn PasswordHasher>) -> Self {
        Self {
            passwords,
            credential_workers: Arc::new(Semaphore::new(2)),
        }
    }
    pub async fn hash_password(&self, password: String) -> Result<String, IdentityError> {
        let password = zeroize::Zeroizing::new(password);
        let permit = Arc::clone(&self.credential_workers)
            .try_acquire_owned()
            .map_err(|_| IdentityError::RateLimited)?;
        let passwords = Arc::clone(&self.passwords);
        // Accepted blocking boundary: the bounded credential permit stays inside
        // the hash job if HTTP is cancelled. This closure has no database/session
        // handles or durable claims; Tokio owns its finite CPU work through exit.
        tokio::task::spawn_blocking(move || {
            let _permit = permit;
            passwords.hash(&password)
        })
        .await
        .map_err(|_| IdentityError::Credential)?
    }

    pub async fn verify_password(
        &self,
        password: String,
        encoded_hash: String,
    ) -> Result<bool, IdentityError> {
        let password = zeroize::Zeroizing::new(password);
        let permit = Arc::clone(&self.credential_workers)
            .try_acquire_owned()
            .map_err(|_| IdentityError::RateLimited)?;
        let passwords = Arc::clone(&self.passwords);
        // Accepted blocking boundary: the bounded credential permit stays inside
        // the hash job if HTTP is cancelled. This closure has no database/session
        // handles or durable claims; Tokio owns its finite CPU work through exit.
        tokio::task::spawn_blocking(move || {
            let _permit = permit;
            passwords.verify(&password, &encoded_hash)
        })
        .await
        .map_err(|_| IdentityError::Credential)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Condvar, Mutex};

    struct ControlledHasher {
        entered: tokio::sync::mpsc::UnboundedSender<()>,
        released: (Mutex<bool>, Condvar),
    }
    impl ControlledHasher {
        fn block(&self) {
            self.entered.send(()).unwrap();
            let (lock, ready) = &self.released;
            let _guard = ready
                .wait_while(lock.lock().unwrap(), |released| !*released)
                .unwrap();
        }
        fn release(&self) {
            *self.released.0.lock().unwrap() = true;
            self.released.1.notify_all();
        }
    }
    impl PasswordHasher for ControlledHasher {
        fn hash(&self, _: &str) -> Result<String, IdentityError> {
            self.block();
            Ok("encoded".into())
        }
        fn verify(&self, _: &str, _: &str) -> bool {
            self.block();
            true
        }
        fn dummy_hash(&self) -> String {
            "dummy".into()
        }
    }

    // A current-thread executor cannot deliver either entered notification if
    // credential work blocks it. Synchronization, rather than timing, gates jobs.
    #[tokio::test]
    async fn shared_capacity_survives_cancellation_until_blocking_work_finishes() {
        let (entered, mut receiver) = tokio::sync::mpsc::unbounded_channel();
        let hasher = Arc::new(ControlledHasher {
            entered,
            released: (Mutex::new(false), Condvar::new()),
        });
        struct ReleaseOnDrop(Arc<ControlledHasher>);
        impl Drop for ReleaseOnDrop {
            fn drop(&mut self) {
                self.0.release();
            }
        }
        let _cleanup = ReleaseOnDrop(hasher.clone());
        let work = Arc::new(CredentialWork::new(hasher.clone()));
        let hash = {
            let work = work.clone();
            tokio::spawn(async move { work.hash_password("password".into()).await })
        };
        let verify = {
            let work = work.clone();
            tokio::spawn(
                async move { work.verify_password("password".into(), "hash".into()).await },
            )
        };
        receiver.recv().await.unwrap();
        receiver.recv().await.unwrap();
        hash.abort();
        assert!(hash.await.unwrap_err().is_cancelled());
        assert!(matches!(
            work.hash_password("third".into()).await,
            Err(IdentityError::RateLimited)
        ));
        assert!(matches!(
            work.verify_password("third".into(), "hash".into()).await,
            Err(IdentityError::RateLimited)
        ));
        hasher.release();
        assert!(verify.await.unwrap().unwrap());
        let permits = work.credential_workers.acquire_many(2).await.unwrap();
        drop(permits);
        assert_eq!(work.hash_password("later".into()).await.unwrap(), "encoded");
    }
}
