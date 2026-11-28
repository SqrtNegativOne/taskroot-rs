//! Managed store handle shared by every command and the loader.

use tokio::sync::{Mutex, MutexGuard};

use super::model::Store;

/// The managed store lock.
///
/// The lock is held only for short in-memory reads, validation snapshots, and
/// revision/ETag-guarded commits — never across `CalDAV` network I/O. That
/// keeps settings reads, reminder polls, and queries responsive while `PUT`,
/// `DELETE`, or `REPORT` requests are in flight.
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
