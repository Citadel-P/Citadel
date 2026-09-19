use serde::Deserialize;

pub(crate) fn optional_name<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<String, D::Error> {
    Ok(Option::<String>::deserialize(deserializer)?.unwrap_or_default())
}

pub(crate) fn enabled_status() -> String {
    "Enabled".into()
}
