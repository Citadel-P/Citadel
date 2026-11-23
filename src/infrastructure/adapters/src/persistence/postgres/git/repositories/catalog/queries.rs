use super::SYNC_STATUS;
use crate::persistence::postgres::authorization::{ALL_REQUESTED_TAGS, authorized_catalog_query};

pub(super) fn authorized_list(tag_names: &[String], scoped: bool) -> String {
    let mut filter = if tag_names.is_empty() {
        String::new()
    } else {
        ALL_REQUESTED_TAGS.to_owned()
    };
    if scoped {
        filter.push_str(if tag_names.is_empty() {
            " AND resource.id=$6"
        } else {
            " AND resource.id=$7"
        });
    }
    authorized_catalog_query(
        "gitrepositories",
        &format!(", {SYNC_STATUS}, NULL::jsonb AS latest_activity"),
        &filter,
    )
}
