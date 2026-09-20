use crate::LookupResourceType as Kind;
use citadel_primitives::ActorId;
use futures_util::future::BoxFuture;
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
pub struct LookupResourceInfo {
    pub id: Uuid,
    pub name: String,
    pub group: Option<String>,
}

#[derive(Debug, Clone, Copy)]
pub struct LookupQuery {
    pub source: Option<Kind>,
    pub source_id: Option<Uuid>,
    pub target: Kind,
    pub platform_id: Option<Uuid>,
}

#[derive(Debug, thiserror::Error)]
pub enum LookupError {
    #[error("{0}")]
    Validation(String),
    #[error("Administrator access is required for this resource lookup.")]
    Forbidden,
    #[error("The source resource does not exist or is not accessible.")]
    NotFound,
    #[error("Lookup storage failed: {0}")]
    Storage(String),
}

impl LookupQuery {
    pub fn validate(&self, administrator: bool) -> Result<(), LookupError> {
        use Kind::*;
        // Preserve the security precedence of the .NET handler.
        if !administrator
            && (self.source == Some(User)
                || matches!(
                    self.target,
                    User | Team | Role | OidcProvider | License | ServiceAccount
                ))
        {
            return Err(LookupError::Forbidden);
        }
        if self.source.is_none() && self.source_id.is_some() {
            return Err(LookupError::Validation(
                "sourceResourceType must be provided when sourceResourceId is specified.".into(),
            ));
        }
        let supported = match self.source {
            None => !matches!(self.target, Network | Volume | GitAccount | AlertChannel),
            Some(Deployment) => matches!(
                self.target,
                Platform | Registry | Image | Network | ResourceBinding
            ),
            Some(Stack) => matches!(
                self.target,
                Platform | Registry | GitRepository | ResourceBinding
            ),
            Some(SwarmService) => self.target == ResourceBinding,
            Some(User) => matches!(self.target, Team | Role),
            Some(Platform) => matches!(
                self.target,
                Deployment | Stack | Registry | Image | Network | Volume
            ),
            Some(Alert) => matches!(
                self.target,
                Platform
                    | Deployment
                    | Stack
                    | GitRepository
                    | SwarmService
                    | AutomationAction
                    | Build
                    | AlertChannel
            ),
            Some(Image) => self.target == Registry && self.source_id.is_none(),
            _ => false,
        };
        if !supported {
            return Err(LookupError::Validation(format!(
                "Lookup from {:?} to {:?} is not supported.",
                self.source, self.target
            )));
        }
        if (self.source == Some(Platform) && matches!(self.target, Image | Network | Volume)
            || self.source == Some(Deployment) && self.target == Image)
            && self.source_id.is_none()
        {
            return Err(LookupError::Validation(
                "sourceResourceId is required for this lookup.".into(),
            ));
        }
        if (self.source.is_none() && self.target == Image
            || self.source == Some(Deployment) && self.target == Network)
            && self.platform_id.is_none_or(|id| id.is_nil())
        {
            return Err(LookupError::Validation(
                "platformId is required for this lookup.".into(),
            ));
        }
        Ok(())
    }
}

pub struct LookupCaller {
    pub actor_id: ActorId,
    pub user_id: Uuid,
    pub administrator: bool,
    pub service_accounts_enabled: bool,
}

pub enum LookupResult {
    Rows(Vec<LookupResourceInfo>),
    // Authorization has completed; these resources are read through the existing
    // Local/Agent runtime rather than inventing a second inventory cache.
    PlatformResources { platform_id: Uuid, kind: Kind },
}

pub trait LookupReader: Send + Sync {
    fn lookup<'a>(
        &'a self,
        caller: &'a LookupCaller,
        request: &'a LookupQuery,
    ) -> BoxFuture<'a, Result<LookupResult, LookupError>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn administrator_gate_precedes_unsupported_pair() {
        for target in [
            Kind::User,
            Kind::Team,
            Kind::Role,
            Kind::License,
            Kind::OidcProvider,
            Kind::ServiceAccount,
        ] {
            let request = LookupQuery {
                source: Some(Kind::Deployment),
                source_id: None,
                target,
                platform_id: None,
            };
            assert!(matches!(
                request.validate(false),
                Err(LookupError::Forbidden)
            ));
            assert!(matches!(
                request.validate(true),
                Err(LookupError::Validation(_))
            ));
        }
    }

    #[test]
    fn context_and_source_ids_are_required_without_rejecting_add_mode() {
        let mut request = LookupQuery {
            source: None,
            source_id: None,
            target: Kind::Image,
            platform_id: None,
        };
        assert!(request.validate(true).is_err());
        request.platform_id = Some(Uuid::now_v7());
        assert!(request.validate(false).is_ok());
        request.source_id = Some(Uuid::now_v7());
        assert!(request.validate(true).is_err());
        request.source = Some(Kind::Deployment);
        request.target = Kind::Registry;
        request.source_id = None;
        assert!(request.validate(false).is_ok());
    }
}

pub mod model;
pub use model::LookupResourceType;
