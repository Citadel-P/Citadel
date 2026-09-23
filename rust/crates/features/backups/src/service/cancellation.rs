use super::*;

impl BackupService {
    pub async fn cancel_backup(&self, id: Uuid) -> Result<(), BackupError> {
        if let Some(t) = self
            .active_backups
            .lock()
            .map_err(poison)?
            .get(&id)
            .cloned()
        {
            t.cancel();
            return Ok(());
        }
        if self.store.cancel_backup(id).await? {
            self.progress(id, "Cancelled", "Backup cancelled.");
            self.changed();
            Ok(())
        } else {
            Err(BackupError::Conflict("Backup Run is not active.".into()))
        }
    }

    pub async fn cancel_restore(&self, id: Uuid) -> Result<(), BackupError> {
        if let Some(t) = self
            .active_restores
            .lock()
            .map_err(poison)?
            .get(&id)
            .cloned()
        {
            t.cancel();
            return Ok(());
        }
        if self.store.cancel_restore(id).await? {
            self.progress(id, "Cancelled", "Restore cancelled.");
            self.changed();
            Ok(())
        } else {
            Err(BackupError::Conflict("Restore Run is not active.".into()))
        }
    }
}
