use crate::api::metadata_patch::MetadataPatch;
use crate::api::resource_access::require_actor;
use crate::identity_http::{IdentityHttpResult, identity_result};
use axum::extract::rejection::PathRejection;
use axum::extract::{Extension, Path};
use axum::http::HeaderMap;
use citadel_identity::ActorPrincipal;
use serde::Deserialize;
use uuid::Uuid;
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DeleteResourcesInput {
    pub(crate) ids: Vec<Uuid>,
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RenameResourceInput {
    pub(crate) id: Uuid,
    pub(crate) name: String,
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PatchResourceMetadataInput {
    #[serde(default)]
    #[schema(value_type = Option<String>, required = false)]
    pub(crate) description: MetadataPatch<String>,
    #[serde(default, rename = "tags")]
    pub(crate) _tags: Option<Vec<String>>,
}

#[derive(Default)]
pub(crate) struct CatalogFilters {
    pub(crate) include_disabled: bool,
    pub(crate) tags: Vec<Uuid>,
}

pub(crate) fn principal_and_id(
    headers: &HeaderMap,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
) -> IdentityHttpResult<(ActorPrincipal, Uuid)> {
    let principal = identity_result(require_actor(principal), headers)?;
    let Path(id) = identity_result(
        path.map_err(|error| citadel_identity::IdentityError::Validation(error.to_string())),
        headers,
    )?;
    Ok((principal, id))
}

pub(crate) fn parse_catalog_filters(
    query: Option<&str>,
    allow_include_disabled: bool,
) -> Result<CatalogFilters, citadel_identity::IdentityError> {
    let mut filters = CatalogFilters::default();
    for (key, value) in url::form_urlencoded::parse(query.unwrap_or_default().as_bytes()) {
        if key == "includeDisabled" && allow_include_disabled {
            filters.include_disabled = value.parse::<bool>().map_err(|_| {
                citadel_identity::IdentityError::Validation(
                    "includeDisabled must be true or false.".to_owned(),
                )
            })?;
        } else if key == "tags" {
            if filters.tags.len() >= 100 {
                return Err(citadel_identity::IdentityError::Validation(
                    "At most 100 Tags may be filtered at once.".to_owned(),
                ));
            }
            let tag = Uuid::parse_str(&value).map_err(|_| {
                citadel_identity::IdentityError::Validation("A Tag ID is invalid.".to_owned())
            })?;
            if !filters.tags.contains(&tag) {
                filters.tags.push(tag);
            }
        }
    }
    Ok(filters)
}
