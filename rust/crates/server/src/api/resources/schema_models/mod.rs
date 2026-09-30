//! OpenAPI belongs to the HTTP boundary. Feature crates own serialization and behavior.
// Values come from native serialization; the exhaustive match also detects added variants.
macro_rules! enum_schema {
    ($schema:ident, $name:literal, $domain:path, [$($variant:ident),+ $(,)?]) => {
        pub struct $schema;
        impl utoipa::ToSchema for $schema {
            fn name() -> std::borrow::Cow<'static, str> { $name.into() }
        }
        impl utoipa::PartialSchema for $schema {
            fn schema() -> utoipa::openapi::RefOr<utoipa::openapi::schema::Schema> {
                use $domain as Domain;
                let values = [$(Domain::$variant),+];
                utoipa::openapi::schema::ObjectBuilder::new()
                    .schema_type(utoipa::openapi::schema::Type::String)
                    .enum_values(Some(values.into_iter().map(|value| {
                        serde_json::to_value(value).expect("closed vocabulary serializes")
                    })))
                    .into()
            }
        }
        const _: fn($domain) = |value| {
            use $domain as Domain;
            match value { $(Domain::$variant => ()),+ }
        };
    };
}

// Declare each object once. The exhaustive destructure makes added/removed native
// fields fail compilation, while assignments also enforce the native field types.
macro_rules! schema_model {
    ($domain:path => $(#[$meta:meta])* pub struct $name:ident {
        $($(#[$field_meta:meta])* pub $field:ident: $ty:ty),* $(,)?
    }) => {
        $(#[$meta])*
        pub struct $name { $($(#[$field_meta])* pub $field: $ty),* }
        impl From<$domain> for $name {
            fn from(value: $domain) -> Self {
                use $domain as Domain;
                let Domain { $($field),* } = value;
                Self { $($field),* }
            }
        }
    };
}

pub mod activities;
pub mod alerts;
pub mod automation;
pub mod backups;
pub mod bindings;
pub mod builds;
pub mod deployments;
pub mod discovery;
pub mod git;
pub mod licensing;
pub mod platforms;
pub mod primitives;
pub mod registries;
pub mod stacks;
pub mod swarm_services;

#[cfg(test)]
mod tests;
