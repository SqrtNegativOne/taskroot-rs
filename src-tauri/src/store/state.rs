//! Managed store handle shared by every command and the sync engine.

use tokio::sync::{Mutex, MutexGuard};

use super::model::Store;

/// The managed store lock.
///
/// A `tokio::sync::Mutex` is used deliberately: an edit holds the store while
/// its `PUT` is in flight, so a concurrent pull can never overwrite the merge
/// with a stale resource. The single-user data set makes the short lock
/// contention acceptable.
pub struct StoreState(pub Mutex<Store>);

impl StoreState {
    #[must_use]
    pub fn new(store: Store) -> Self {
        Self(Mutex::new(store))
    }

    pub async fn lock(&self) -> MutexGuard<'_, Store> {
        self.0.lock().await
    }

    pub async fn read<T>(&self, project: impl FnOnce(&Store) -> T) -> T {
        let guard = self.0.lock().await;
        project(&guard)
    }

    pub async fn write<T>(&self, mutate: impl FnOnce(&mut Store) -> T) -> T {
        let mut guard = self.0.lock().await;
        mutate(&mut guard)
    }
}
