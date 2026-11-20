use super::SYNC_STATUS;
use crate::persistence::postgres::authorization::{ALL_REQUESTED_TAGS, authorized_catalog_query};
use uuid::Uuid;

pub(super) fn authorized_list(tag_ids: &[Uuid]) -> String {
    authorized_catalog_query(
        "gitrepositories",
        &format!(", {SYNC_STATUS}, NULL::jsonb AS latest_activity"),
        if tag_ids.is_empty() {
            ""
        } else {
            ALL_REQUESTED_TAGS
        },
    )
}
