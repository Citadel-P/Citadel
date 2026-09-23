//! Frozen schemas at the remaining dynamic JSON compatibility boundary.
//! New contracts use real DTOs with `ToSchema`.
use std::{collections::BTreeMap, sync::LazyLock};
use utoipa::{
    PartialSchema, ToSchema,
    openapi::{RefOr, schema::Schema},
};

fn all() -> &'static BTreeMap<String, RefOr<Schema>> {
    static SCHEMAS: LazyLock<BTreeMap<String, RefOr<Schema>>> = LazyLock::new(|| {
        serde_json::from_str(include_str!("compatibility.json"))
            .expect("compatibility schemas are valid OpenAPI")
    });
    &SCHEMAS
}

pub fn schemas() -> BTreeMap<String, RefOr<Schema>> {
    all().clone()
}

// The stats query is decoded as an integer and validated by StatsWindow, rather
// than a serde enum. Preserve its existing 24/48/72 hour schema at that boundary.
pub struct StatsHours;
impl PartialSchema for StatsHours {
    fn schema() -> RefOr<Schema> {
        all()["StatsHours"].clone()
    }
}
impl ToSchema for StatsHours {}
