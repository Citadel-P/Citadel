use super::{ApiError, field_error};

/// Limit the encoded query before decoding repeated or multibyte values.
pub(crate) fn parse_tag_filters(query: Option<&str>) -> Result<Vec<String>, ApiError> {
    let query = query.unwrap_or_default();
    let invalid = |message: &str| field_error("$.tags".into(), message.into());
    if query.len() > 8192 {
        return Err(invalid("Tag filter query exceeds 8192 bytes."));
    }
    citadel_tags::tag_filter_names(
        url::form_urlencoded::parse(query.as_bytes())
            .filter(|(key, _)| key.eq_ignore_ascii_case("tags"))
            .map(|(_, name)| name),
    )
    .map_err(|error| invalid(&error.to_string()))
}

#[cfg(test)]
mod tests {
    use crate::realtime_groups::Group;
    use serde_json::json;
    use uuid::Uuid;
    #[test]
    fn unicode_and_punctuation_have_the_same_http_and_websocket_budget() {
        let names = (0..5)
            .map(|i| format!("{}+,&:{i}%", "é".repeat(120)))
            .collect::<Vec<_>>();
        let uri = |names: &[String]| {
            let mut query = url::form_urlencoded::Serializer::new(String::new());
            query.extend_pairs(names.iter().map(|name| ("tags", name)));
            query.finish()
        };
        let join = |names: &[String]| {
            Group::from_argument(&json!({
                "subscriptionId": Uuid::now_v7(), "group":"scripts", "tags":names,
            }))
        };
        assert_eq!(
            crate::api::request_validation::parse_tag_filters(Some(&uri(&names))).unwrap(),
            citadel_tags::tag_filter_names(&names).unwrap()
        );
        assert!(join(&names).is_some());
        let mut oversized = names;
        oversized.push("é".repeat(120));
        assert!(crate::api::request_validation::parse_tag_filters(Some(&uri(&oversized))).is_err());
        assert!(join(&oversized).is_none());
    }
}
