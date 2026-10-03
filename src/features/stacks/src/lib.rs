#![forbid(unsafe_code)]

mod compose;
mod model;
mod service;
mod updates;
mod webhooks;

mod repository;
mod runtime;

pub mod permissions;

mod tasks;
pub use tasks::StackTaskSpawner;

mod commands;

mod read_models;

pub use model::{
    ComposeProjectImportValidation, ComposeProjectRuntimeService, ComposeProjectServiceComparison,
    ImageUpdateState, RecreateStackOnNewCommitState, RecreateStackOnNewImageState,
    ResolvedStackBindings, ResolvedStackBuildImageBinding, ResourceBindingSnapshot, Stack,
    StackAction, StackAdoptionIssue, StackApplyEventType, StackApplySource, StackBinding,
    StackBuildImageBinding, StackCommand, StackDeletionClaim, StackDrift, StackDriftMode,
    StackDriftMonitorFailure, StackDriftMonitorResult, StackDriftPolicy, StackDriftReport,
    StackError, StackImportClaim, StackImportKind, StackOperationClaim, StackOrchestrationMode,
    StackProgressItem, StackReconciliationAction, StackReconciliationActionType,
    StackReconciliationResult, StackReconciliationStatus, StackRelease, StackReleaseSource,
    StackReleaseStatus, StackRuntimeContainer, StackRuntimeResult, StackRuntimeService,
    StackRuntimeSnapshot, StackSource, StackSourceFile, StackSpec, StackSpecCommon,
    StackStateClaim, StackUpdateBehavior, StackUpdateState, StackWebhookConfig,
    normalize_project_name,
};

pub use commands::{
    ApplyStack, CreateStack, DuplicateStackSource, ImportComposeProject, RenameStack,
    RollbackStack, StackApplyOptions, UpdateStack,
};

pub use read_models::{
    ComposeProjectImportDraft, ComposeProjectImportSource, ComposeProjectStackDraft,
    DuplicateDraftWarning, StackConfig, StackDraft, StackDuplicateDraft, StackFilter,
};

pub use compose::{
    ComposeModel, ComposeService, SwarmStackCompatibilityIssue, SwarmStackCompatibilityReport,
    SwarmStackCompatibilitySeverity, analyze_swarm_compatibility, compose_digest,
    create_ownership_labels_override, inject_ownership_labels, parse_compose,
};

pub use service::{
    NoopStackChangeNotifier, StackChangeNotifier, StackProgress, StackService, StackUpdateScanner,
    calculate_drift, import_runtime_fingerprint,
};

pub use repository::StackRepository;

pub use runtime::{
    StackBindingResolverPort, StackBuildImageResolverPort, StackRuntime,
    StackSourceMaterializerPort,
};

pub use updates::{
    ManualStackImageCheck, ManualStackUpdateEvaluation, StackImageKey, StackImageUpdateItem,
    build_manual_stack_checks, evaluate_manual_stack_updates, state_key,
};

pub use webhooks::{
    StackEntitlements, StackWebhookJob, can_queue_stack_webhook, stack_git_path_matches,
    stack_webhook_fingerprint,
};

use model::*;
