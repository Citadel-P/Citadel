use super::super::{
    CheckedUpdateInstallation, FinalizeUpdateRequest, JournalDirectory, LaunchedUpdateHelper,
    check_registered_compose_installation, finalize_update, launch_update_helper,
};
use citadel_execution::ProcessRunner;
use citadel_updates::{UpdateError, UpdateJournal, UpdateOperation};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio_util::sync::CancellationToken;

#[derive(Serialize, Deserialize)]
#[serde(
    tag = "action",
    content = "request",
    rename_all = "camelCase",
    deny_unknown_fields
)]
pub enum HostUpdateRequest {
    Startup { state_directory: PathBuf },
    Check { state_directory: PathBuf },
    CheckDatabase(Box<citadel_updates::UpdatePlan>),
    Launch(Box<UpdateOperation>),
    Finalize(Box<FinalizeUpdateRequest>),
    Completed(Box<UpdateOperation>),
    FailedJournal(Box<UpdateOperation>),
    RecoverSource(Box<UpdateOperation>),
}

#[derive(Serialize, Deserialize)]
#[serde(
    tag = "status",
    content = "result",
    rename_all = "camelCase",
    deny_unknown_fields
)]
pub enum HostUpdateResponse {
    Registered(Box<super::super::RegisteredComposeInstallation>),
    Checked(Box<CheckedUpdateInstallation>),
    DatabaseChecked {
        plan_id: uuid::Uuid,
        blocker: Option<String>,
    },
    Launched(Box<LaunchedUpdateHelper>),
    Finalized(Box<UpdateJournal>),
    FailedJournal(Box<UpdateJournal>),
    SourceRecovered(Box<UpdateOperation>),
    Completed(Box<super::super::CompletedUpdateTarget>),
}

pub async fn execute_host_update(
    request: HostUpdateRequest,
    process: &dyn ProcessRunner,
    cancellation: &CancellationToken,
) -> Result<HostUpdateResponse, UpdateError> {
    match request {
        HostUpdateRequest::CheckDatabase(plan) => {
            let blocker = match super::super::handoff::check_database_eligibility(
                &plan,
                process,
                cancellation,
            )
            .await
            {
                Ok(()) => None,
                // Only curated policy conflicts are returned; process output stays private.
                Err(UpdateError::Conflict(message)) => Some(message),
                Err(error) => return Err(error),
            };
            Ok(HostUpdateResponse::DatabaseChecked {
                plan_id: plan.id,
                blocker,
            })
        }
        HostUpdateRequest::RecoverSource(operation) => {
            super::super::handoff::recover_source(&operation, process, cancellation).await?;
            Ok(HostUpdateResponse::SourceRecovered(operation))
        }

        HostUpdateRequest::FailedJournal(operation) => {
            Ok(HostUpdateResponse::FailedJournal(Box::new(
                super::super::handoff::failed_journal(&operation, process, cancellation).await?,
            )))
        }
        HostUpdateRequest::Completed(operation) => Ok(HostUpdateResponse::Completed(Box::new(
            super::super::completed_update_target(&operation, process, cancellation).await?,
        ))),
        HostUpdateRequest::Startup { state_directory } => Ok(HostUpdateResponse::Registered(
            Box::new(super::super::check_startup_registration(&state_directory)?),
        )),
        HostUpdateRequest::Check { state_directory } => Ok(HostUpdateResponse::Checked(Box::new(
            check_registered_compose_installation(&state_directory, process, cancellation).await?,
        ))),
        HostUpdateRequest::Launch(operation) => {
            let state = JournalDirectory::open(operation.plan.checkpoint_directory.clone().into())?;
            Ok(HostUpdateResponse::Launched(Box::new(
                launch_update_helper(&state, &operation, process, cancellation).await?,
            )))
        }
        HostUpdateRequest::Finalize(request) => Ok(HostUpdateResponse::Finalized(Box::new(
            finalize_update(&request, process, cancellation).await?,
        ))),
    }
}
