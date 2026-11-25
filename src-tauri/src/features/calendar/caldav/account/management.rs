//! Profile removal/deactivation, separate from network session setup.

use super::service::AccountService;
use crate::error::AppError;
use crate::features::calendar::{Store, StoreState};

impl AccountService {
    /// Deactivate without forgetting saved profiles or passwords.
    /// # Errors
    /// Returns registry storage failures.
    pub async fn disconnect(&self, store: &StoreState) -> Result<(), AppError> {
        let _guard = self.gate.write().await;
        let mut registry = self.repository.read()?;
        registry.active_id = None;
        let mut store = store.lock().await;
        self.repository.write(&registry)?;
        clear(&mut store);
        Ok(())
    }

    /// Forget one profile; removing the active profile leaves no active account.
    /// # Errors
    /// Returns missing-profile, credential or storage failures.
    pub async fn remove(&self, store: &StoreState, id: &str) -> Result<(), AppError> {
        let _guard = self.gate.write().await;
        let mut registry = self.repository.read()?;
        registry.profile(id)?;
        let previous = self.credentials.load(id)?;
        self.credentials.delete(id)?;
        registry.profiles.retain(|profile| profile.id != id);
        let active = registry.active_id.as_deref() == Some(id);
        if active {
            registry.active_id = None;
        }
        let mut store = store.lock().await;
        if let Err(error) = self.repository.write(&registry) {
            self.restore_password(id, previous.as_deref());
            return Err(error);
        }
        if active {
            clear(&mut store);
        }
        Ok(())
    }
}

fn clear(store: &mut Store) {
    store.calendars.clear();
    store.clear_resources();
}
