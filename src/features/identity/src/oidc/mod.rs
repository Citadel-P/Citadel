use crate::Clock;
use crate::IdentityError;
use crate::IdentityService;
use crate::PatchField;
use crate::SecretProtector;
use crate::SessionMetadata;
use crate::SessionTokens;
use crate::UserAuthentication;
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use chrono::{DateTime, Duration, Utc};
use citadel_primitives::ActorId;
use futures_util::future::BoxFuture;
use getrandom::fill;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::sync::Arc;
use url::Url;
use uuid::Uuid;

mod read_models;

mod commands;

mod repository;
pub use repository::{OidcProtocol, OidcStore};
mod service;

pub mod model;

pub use model::{
    DEFAULT_OIDC_SCOPES, OidcDiscovery, OidcExternalLogin, OidcIdentity, OidcLoginState,
    OidcProvider, OidcProviderValidationError,
};

pub use commands::{
    CreateOidcProvider, PatchOidcProvider, PatchOidcProviderMetadata, RenameOidcProvider,
    TestOidcDiscovery,
};

pub use read_models::{
    OidcDiscoveryDetails, OidcLoginComplete, OidcLoginProviderDetails, OidcLoginProvidersDetails,
    OidcLoginStart, OidcProviderDetails, OidcProvidersDetails,
};

pub use service::{OidcService, build_authorization_url, hash_opaque_value, normalize_return_url};
