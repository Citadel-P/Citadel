use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use citadel_identity::IdentityError;
use uuid::Uuid;

use crate::identity_http::IdentityHttpError;

/// Shared collection filters for Deployments, Stacks and Swarm Services.
/// Repeated tags, case-insensitive keys and duplicate handling are compatibility
/// requirements that ordinary serde_urlencoded/Query does not implement.
#[derive(Debug, Default)]
pub(crate) struct WorkloadQuery {
    pub tags: Vec<String>,
    pub platform_id: Option<Uuid>,
}

impl WorkloadQuery {
    pub(crate) fn parse(query: Option<&str>) -> Result<Self, IdentityError> {
        let mut filter = Self::default();
        for (key, value) in url::form_urlencoded::parse(query.unwrap_or_default().as_bytes()) {
            if key.eq_ignore_ascii_case("tags") {
                if filter.tags.len() >= 100 {
                    return Err(super::field_error(
                        "$.tags".into(),
                        "At most 100 tags may be filtered at once.".into(),
                    ));
                }
                if !value.trim().is_empty() && !filter.tags.iter().any(|tag| tag == &value) {
                    filter.tags.push(value.into_owned());
                }
            } else if key.eq_ignore_ascii_case("platformId") {
                let id = Uuid::parse_str(&value).map_err(|_| {
                    super::field_error(
                        "$.platformId".into(),
                        "The Platform ID filter is invalid.".into(),
                    )
                })?;
                if filter.platform_id.replace(id).is_some() {
                    return Err(super::field_error(
                        "$.platformId".into(),
                        "The Platform ID filter may be supplied only once.".into(),
                    ));
                }
            }
        }
        Ok(filter)
    }
}

impl<S: Send + Sync> FromRequestParts<S> for WorkloadQuery {
    type Rejection = IdentityHttpError;

    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        Self::parse(parts.uri.query())
            .map_err(|error| IdentityHttpError::from_parts(error, &parts.headers))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_repeated_case_insensitive_filters_and_ignores_empty_tags() {
        let id = Uuid::now_v7();
        let filter = WorkloadQuery::parse(Some(&format!(
            "TaGs=prod&TAGS=blue&tags=prod&tags=%20&PlatformID={id}&unknown=ignored"
        )))
        .unwrap();
        assert_eq!(filter.tags, ["prod", "blue"]);
        assert_eq!(filter.platform_id, Some(id));
    }

    #[test]
    fn rejects_invalid_or_duplicate_platforms_and_excess_tags() {
        let id = Uuid::now_v7();
        assert!(WorkloadQuery::parse(Some("platformId=invalid")).is_err());
        assert!(WorkloadQuery::parse(Some(&format!("platformId={id}&PLATFORMID={id}"))).is_err());
        let tags = (0..100)
            .map(|i| format!("tags={i}"))
            .collect::<Vec<_>>()
            .join("&");
        assert_eq!(WorkloadQuery::parse(Some(&tags)).unwrap().tags.len(), 100);
        assert!(WorkloadQuery::parse(Some(&format!("{tags}&tags=100"))).is_err());
    }
}
