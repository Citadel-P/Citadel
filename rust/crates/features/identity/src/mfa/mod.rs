use crate::ActorPrincipal;
use crate::Clock;
use crate::IdentityError;
use crate::IdentityService;
use crate::InitializeCitadel;
use crate::Login;
use crate::LoginNextStep;
use crate::LoginOutcome;
use crate::PreparedSession;
use crate::SessionMetadata;
use crate::SessionTokens;
use crate::UserAuthentication;
use chrono::{DateTime, Duration, Utc};
use citadel_activities::{ActivityEvent, ActivityEventInfo};
use citadel_primitives::ActorId;

use futures_util::future::BoxFuture;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;
use zeroize::Zeroizing;

mod read_models;

mod commands;

mod repository;
pub use repository::{MfaStore, RecoveryCodeService, SecretProtector, TotpService};
mod service;

pub mod model;

pub use model::{
    MfaChallenge, MfaConfiguration, MfaCredentialAcceptance, MfaPolicy, MfaSetupSession, TotpSetup,
    UserMfaRecoveryCode, UserMfaSettings,
};

pub use commands::{
    ChallengeCompletionCommit, ConfirmMandatoryMfaSetupInput, ConfirmProfileMfaSetupInput,
    DisableMfaCommit, DisableProfileMfaInput, MandatoryEnrollmentCommit, MfaVerificationInput,
    ProfileEnrollmentCommit, RegenerateProfileMfaRecoveryCodesInput, RegenerateRecoveryCodesCommit,
    ResetMfaCommit, StartProfileMfaSetupInput,
};

pub use read_models::{
    BrowserAuthenticationAction, BrowserAuthenticationResult, MandatoryMfaSetupCompleteDetails,
    MandatoryMfaSetupDetails, MfaVerificationDetails, ProfileMfaRecoveryCodesDetails,
    ProfileMfaSetupDetails, ProfileMfaStatusDetails,
};

pub use service::MfaService;
