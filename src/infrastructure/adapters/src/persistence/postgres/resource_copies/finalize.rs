use super::operations::{cancel_queued, owned_plan, progress};
use super::*;
impl PostgresCopyRepository {
    pub(super) async fn advance_copy(
        &self,
        actor: ActorId,
        id: Uuid,
        storage_result: Option<Result<(), String>>,
    ) -> Result<CopyProgress, CopyError> {
        let access = self.access(actor).await;
        let mut tx = self.pool.begin().await.map_err(storage_error)?;
        sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ")
            .execute(&mut *tx)
            .await
            .map_err(storage_error)?;
        let (plan, status) = owned_plan(&mut tx, actor, id).await?;
        let current = progress(&mut tx, actor, id).await?;
        if status.terminal() || matches!(status, CopyStatus::Review | CopyStatus::Cleaning) {
            return Ok(current);
        }
        let failed = current.restores.iter().any(|r| {
            !matches!(
                r.status.as_str(),
                "Queued" | "Preparing" | "Running" | "Succeeded"
            )
        });
        let authorization = match &access {
            Ok(access) => match self.verify_source(&mut tx, access, &plan).await {
                Ok(()) => self.authorize_current(&mut tx, access, &plan).await,
                Err(error) => Err(error),
            },
            Err(_) => Err(CopyError::NotFound),
        };
        let stopping = status == CopyStatus::Cancelling || failed || authorization.is_err();
        if stopping {
            cancel_queued(&mut tx, id).await?;
            let running = current
                .restores
                .iter()
                .any(|r| matches!(r.status.as_str(), "Preparing" | "Running"));
            let status = if running {
                "Cancelling"
            } else if status == CopyStatus::Cancelling
                && current.message.as_deref() == Some("Cancellation requested.")
            {
                "Cancelled"
            } else {
                "Failed"
            };
            let message = if status == "Cancelling"
                && current.message.as_deref() == Some("Cancellation requested.")
            {
                "Cancellation requested."
            } else if authorization.is_err() {
                "Source or destination permissions, configuration, placement or references changed. No workload was created. Retained destination volumes are listed below."
            } else {
                "Copy stopped. No workload was created. Completed destination volumes are retained; failed restores may have removed their partial volumes."
            };
            sqlx::query("UPDATE resourcecopies SET status=$2,message=$3,updatedat=CURRENT_TIMESTAMP WHERE id=$1").bind(id).bind(status).bind(message).execute(&mut *tx).await.map_err(storage_error)?;
        } else if let Some(storage_result) = storage_result
            && !current.restores.is_empty()
            && current.restores.iter().all(|r| r.status == "Succeeded")
        {
            // No storage mutations occur during finalization. Failure retains the
            // child receipts; retry never repeats a completed restore.
            let access = access.as_ref().map_err(|_| CopyError::NotFound)?;
            // The service verified these immutable child receipts before this
            // transaction. Recheck parent/child state to fence cancellation.
            if let Err(error) = storage_result {
                sqlx::query("UPDATE resourcecopies SET status='Failed',message=$2,updatedat=CURRENT_TIMESTAMP WHERE id=$1").bind(id).bind(error).execute(&mut *tx).await.map_err(storage_error)?;
            } else {
                // A savepoint ensures a name/reference conflict cannot leave half
                // a workload behind while we persist the terminal copy failure.
                let mut creation = tx.begin().await.map_err(storage_error)?;
                match create_destination(&mut creation, access, &plan).await {
                    Ok(destination) => {
                        sqlx::query("UPDATE resourcecopies SET status='Succeeded',destinationid=$2,message='Copy created. Review its settings and Apply when ready.',updatedat=CURRENT_TIMESTAMP WHERE id=$1").bind(id).bind(destination).execute(&mut *creation).await.map_err(storage_error)?;
                        creation.commit().await.map_err(storage_error)?;
                    }
                    Err(error) => {
                        creation.rollback().await.map_err(storage_error)?;
                        let error = error.to_string();
                        sqlx::query("UPDATE resourcecopies SET status='Failed',message=$2,updatedat=CURRENT_TIMESTAMP WHERE id=$1").bind(id).bind(error).execute(&mut *tx).await.map_err(storage_error)?;
                    }
                }
            }
        } else {
            sqlx::query("UPDATE resourcecopies SET updatedat=CURRENT_TIMESTAMP WHERE id=$1")
                .bind(id)
                .execute(&mut *tx)
                .await
                .map_err(storage_error)?;
        }
        let result = progress(&mut tx, actor, id).await?;
        if result.status != current.status
            || result.message != current.message
            || result.destination_id != current.destination_id
        {
            changes::publish(&mut tx, id).await.map_err(storage_error)?;
        }
        tx.commit().await.map_err(storage_error)?;
        Ok(result)
    }
}
use sqlx::Acquire;
async fn create_destination(
    tx: &mut Transaction<'_, Postgres>,
    access: &Access,
    plan: &CopyPlan,
) -> Result<Uuid, CopyError> {
    let id = Uuid::now_v7();
    let source = &plan.source;
    match &plan.spec {
        CopySpec::Deployment(spec)=>super::super::deployments::create_in_transaction(tx,access.actor,access.administrator,id,&citadel_deployments::CreateDeployment {
            name:plan.request.name.clone(),platform_id:plan.target_platform(),description:source.description.clone(),spec:*spec.clone(),tag_ids:source.tag_ids.clone(),duplicate_source:Some(citadel_deployments::DuplicateSource { resource_id:source.id,resource_type:"Deployment".into(),resource_name:source.name.clone() }),
        }).await.map_err(|_|conflict("The Deployment could not be created. Check its name, references and permissions; restored volumes were retained."))?,
        CopySpec::Stack(spec)=>super::super::stacks::create_in_transaction(tx,access.actor,access.administrator,id,&citadel_stacks::CreateStack {
            name:plan.request.name.clone(),platform_id:plan.target_platform(),description:source.description.clone(),stack_source:spec.source(),spec:*spec.clone(),drift_policy:Some(citadel_stacks::StackDriftPolicy::default().for_duplicate()),tag_ids:source.tag_ids.clone(),duplicate_source:Some(citadel_stacks::DuplicateStackSource { id:source.id,name:source.name.clone() }),
        }).await.map_err(|_|conflict("The Stack could not be created. Check its name, references and permissions; restored volumes were retained."))?,
        CopySpec::SwarmService(spec)=>super::super::swarm_services::create_in_transaction(tx,access.actor,access.administrator,id,&citadel_swarm_services::CreateSwarmService {
            name:plan.request.name.clone(),platform_id:plan.target_platform(),description:source.description.clone(),spec:*spec.clone(),tag_ids:source.tag_ids.clone(),duplicate_source:Some(citadel_swarm_services::SwarmServiceDuplicateSource { resource_id:source.id,resource_type:"SwarmService".into(),resource_name:source.name.clone() }),
        }).await.map_err(|_|conflict("The Swarm Service could not be created. Check its name, references and permissions; restored volumes were retained."))?,
    }
    Ok(id)
}
