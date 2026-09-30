//! The activity API applies the same casing projection to its native schema and payload.
use utoipa::{
    PartialSchema, ToSchema,
    openapi::{RefOr, schema::Schema},
};

pub struct PublicActivityEventInfo;
impl PartialSchema for PublicActivityEventInfo {
    fn schema() -> RefOr<Schema> {
        public_schema(
            crate::api::resources::schema_models::activities::ActivityEventInfoSchema::schema(),
        )
    }
}
impl ToSchema for PublicActivityEventInfo {
    fn schemas(schemas: &mut Vec<(String, RefOr<Schema>)>) {
        let mut native = Vec::new();
        crate::api::resources::schema_models::activities::ActivityEventInfoSchema::schemas(
            &mut native,
        );
        schemas.extend(
            native
                .into_iter()
                .map(|(name, schema)| (name, public_schema(schema))),
        );
    }
}
fn public_schema(mut schema: RefOr<Schema>) -> RefOr<Schema> {
    fn project(schema: &mut RefOr<Schema>) {
        use utoipa::openapi::schema::{AdditionalProperties, ArrayItems};
        let RefOr::T(schema) = schema else {
            return;
        };
        match schema {
            Schema::Object(object) => {
                object.properties = std::mem::take(&mut object.properties)
                    .into_iter()
                    .map(|(key, mut value)| {
                        project(&mut value);
                        (super::presentation::camel_case_key(key), value)
                    })
                    .collect();
                for key in &mut object.required {
                    *key = super::presentation::camel_case_key(std::mem::take(key));
                }
                if let Some(properties) = &mut object.additional_properties
                    && let AdditionalProperties::RefOr(schema) = properties.as_mut()
                {
                    project(schema);
                }
            }
            Schema::Array(array) => {
                if let ArrayItems::RefOrSchema(schema) = &mut array.items {
                    project(schema);
                }
            }
            Schema::OneOf(one) => one.items.iter_mut().for_each(project),
            Schema::AllOf(all) => all.items.iter_mut().for_each(project),
            Schema::AnyOf(any) => any.items.iter_mut().for_each(project),
            _ => {}
        }
    }
    project(&mut schema);
    schema
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    #[test]
    fn public_event_schema_tracks_native_variants_and_snapshot_casing() {
        let mut dependencies = Vec::new();
        PublicActivityEventInfo::schemas(&mut dependencies);
        let dependencies: std::collections::BTreeMap<_, _> = dependencies.into_iter().collect();
        let snapshot = serde_json::to_value(&dependencies["DeploymentActivitySnapshot"]).unwrap();
        assert!(snapshot["properties"].get("platformId").is_some());
        assert!(snapshot["properties"].get("PlatformId").is_none());
        assert!(
            snapshot["required"]
                .as_array()
                .unwrap()
                .contains(&json!("platformId"))
        );

        let native = serde_json::to_value(
            crate::api::resources::schema_models::activities::ActivityEventInfoSchema::schema(),
        )
        .unwrap();
        let public = serde_json::to_value(PublicActivityEventInfo::schema()).unwrap();
        assert_eq!(
            native["oneOf"].as_array().unwrap().len(),
            public["oneOf"].as_array().unwrap().len()
        );
        let event = citadel_activities::ActivityEventInfo::UserSessionRevoked {
            session_id: uuid::Uuid::nil(),
        };
        let stored = serde_json::to_value(event).unwrap();
        assert!(stored.get("SessionId").is_some());
        let payload = super::super::presentation::public_activity_info(stored);
        let schema = public["oneOf"]
            .as_array()
            .unwrap()
            .iter()
            .find(|variant| variant["properties"]["$type"]["enum"] == json!(["UserSessionRevoked"]))
            .expect("native variant must be in the public schema");
        for field in schema["required"].as_array().unwrap() {
            assert!(
                payload.get(field.as_str().unwrap()).is_some(),
                "missing {field}"
            );
        }
        assert_eq!(payload["sessionId"], json!(uuid::Uuid::nil()));
        assert_eq!(payload["$type"], Value::from("UserSessionRevoked"));
    }
}
