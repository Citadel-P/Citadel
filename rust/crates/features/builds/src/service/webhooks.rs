use super::*;

impl BuildService {
    pub async fn queue_webhook(
        &self,
        project: &BuildProject,
        branch: &str,
        commit: Option<&str>,
    ) -> Result<(), BuildError> {
        self.ensure_execution_entitlements(project, "Webhook")
            .await?;
        self.store.enqueue_webhook(project, branch, commit).await?;
        self.changed();
        Ok(())
    }
}
