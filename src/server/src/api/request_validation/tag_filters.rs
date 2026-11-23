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
