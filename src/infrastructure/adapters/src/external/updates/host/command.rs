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
    AutomaticStartup {
        setup: super::super::AutomaticComposeSetup,
        allow_initialize: bool,
    },
    RegisterAutomatic(super::super::AutomaticComposeSetup),
    Startup {
        state_directory: PathBuf,
    },
    Check {
        state_directory: PathBuf,
    },
    CheckReplacement(Box<citadel_updates::UpdatePlan>),
    Launch(Box<UpdateOperation>),
    Finalize(Box<FinalizeUpdateRequest>),
    Completed(Box<UpdateOperation>),
    FailedJournal(Box<UpdateOperation>),
    RecoverSource(Box<UpdateOperation>),
}

impl HostUpdateRequest {
    pub(super) fn cleanup_operation(&self) -> Option<&UpdateOperation> {
        match self {
            Self::Finalize(request) => Some(&request.intent.operation),
            Self::Completed(operation) => Some(operation),
            _ => None,
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(
    tag = "status",
    content = "result",
    rename_all = "camelCase",
    deny_unknown_fields
)]
pub enum HostUpdateResponse {
    ReviewBlocked {
        reason: String,
    },
    RegistrationPending,
    Registered(Box<super::super::RegisteredComposeInstallation>),
    Checked(Box<CheckedUpdateInstallation>),
    ReplacementChecked {
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
        HostUpdateRequest::AutomaticStartup {
            setup,
            allow_initialize,
        } => {
            match super::super::bootstrap::prepare_automatic_setup(
                &setup,
                allow_initialize,
                process,
                cancellation,
            )
            .await?
            {
                Some(registration) => Ok(HostUpdateResponse::Registered(Box::new(registration))),
                None => Ok(HostUpdateResponse::RegistrationPending),
            }
        }
        HostUpdateRequest::RegisterAutomatic(setup) => {
            Ok(HostUpdateResponse::Registered(Box::new(
                super::super::bootstrap::complete_automatic_setup(&setup, process, cancellation)
                    .await?,
            )))
        }
        HostUpdateRequest::CheckReplacement(plan) => {
            let blocker = match super::super::handoff::check_replacement_eligibility(
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
            Ok(HostUpdateResponse::ReplacementChecked {
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
        HostUpdateRequest::Check { state_directory } => {
            match check_registered_compose_installation(&state_directory, process, cancellation)
                .await
            {
                Ok(checked) => Ok(HostUpdateResponse::Checked(Box::new(checked))),
                // Preflight conflicts contain operator-facing validation messages.
                // Docker failures and process output stay behind the host boundary.
                Err(UpdateError::Conflict(reason)) => {
                    Ok(HostUpdateResponse::ReviewBlocked { reason })
                }
                Err(error) => Err(error),
            }
        }
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
