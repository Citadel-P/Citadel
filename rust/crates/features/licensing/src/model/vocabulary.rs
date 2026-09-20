use serde::{Deserialize, Serialize};
macro_rules! database_string_enum {
    ($(#[$metadata:meta])* pub enum $name:ident { $($(#[$variant_metadata:meta])* $variant:ident),+ $(,)? }) => {
        $(#[$metadata])*
        pub enum $name {
            $($(#[$variant_metadata])* $variant),+
        }

        impl $name {
            pub const ALL: &'static [Self] = &[$(Self::$variant),+];

            #[must_use]
            pub const fn as_database_str(self) -> &'static str {
                match self {
                    $(Self::$variant => stringify!($variant)),+
                }
            }

            #[must_use]
            pub fn from_database_str(value: &str) -> Option<Self> {
                match value {
                    $(stringify!($variant) => Some(Self::$variant)),+,
                    _ => None,
                }
            }
        }
    };
}
database_string_enum! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, utoipa::ToSchema)]
    pub enum LicenseCapability {
        CustomAccessControl,
        AutomatedOperations,
        AdvancedAlerting,
        OperationalGuardrails,
        ElasticBuildExecution,
    }
}

impl LicenseCapability {
    #[must_use]
    pub const fn as_license_key(self) -> &'static str {
        match self {
            Self::CustomAccessControl => "custom-access-control",
            Self::AutomatedOperations => "automated-operations",
            Self::AdvancedAlerting => "advanced-alerting",
            Self::OperationalGuardrails => "operational-guardrails",
            Self::ElasticBuildExecution => "elastic-build-execution",
        }
    }

    #[must_use]
    pub fn from_license_key(value: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .copied()
            .find(|capability| capability.as_license_key() == value)
    }
}

database_string_enum! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, utoipa::ToSchema)]
    pub enum LicenseStatus {
        Community,
        Valid,
        GracePeriod,
        NotYetValid,
        Expired,
        Invalid,
        InstanceMismatch,
        UnsupportedSchema,
        UnknownSigningKey,
    }
}
