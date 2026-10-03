//! Helpers for feature-owned status vocabularies stored as text.

/// Declare a status vocabulary with exact, fallible storage parsing and stable wire names.
#[macro_export]
macro_rules! status_enum {
    (pub enum $name:ident { $($variant:ident),+ $(,)? }) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
        pub enum $name { $($variant),+ }

        impl $name {
            pub const ALL: &'static [Self] = &[$(Self::$variant),+];
            pub const fn as_str(self) -> &'static str {
                match self { $(Self::$variant => stringify!($variant)),+ }
            }
        }
        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(self.as_str())
            }
        }
        impl std::str::FromStr for $name {
            type Err = String;
            fn from_str(value: &str) -> Result<Self, Self::Err> {
                match value {
                    $(stringify!($variant) => Ok(Self::$variant)),+,
                    _ => Err(format!("Unknown {} '{value}'.", stringify!($name))),
                }
            }
        }
    };
}

#[cfg(test)]
mod tests {
    status_enum! { pub enum TestStatus { Queued, Running, Succeeded, Failed } }

    #[test]
    fn status_names_roundtrip_through_storage_and_json_without_normalization() {
        for status in TestStatus::ALL {
            let name = status.as_str();
            assert_eq!(name.parse::<TestStatus>().unwrap(), *status);
            assert_eq!(status.to_string(), name);
            let json = serde_json::to_value(status).unwrap();
            assert_eq!(json, serde_json::Value::String(name.into()));
            assert_eq!(serde_json::from_value::<TestStatus>(json).unwrap(), *status);
        }
        for invalid in ["", "Unknown", "succeeded", "Failed ", " Running"] {
            assert!(invalid.parse::<TestStatus>().is_err(), "{invalid:?}");
            assert!(serde_json::from_value::<TestStatus>(serde_json::json!(invalid)).is_err());
        }
    }
}
