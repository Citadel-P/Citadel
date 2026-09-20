//! Transport-independent authorization vocabulary. Transitional owner until Phase 11.
use crate::PermissionLevel;
use crate::ResourceType;
use crate::SpecificPermission;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PermissionRequirement {
    pub resource_type: ResourceType,
    pub level: PermissionLevel,
    pub specific: Option<SpecificPermission>,
}

pub trait PermissionPolicy {
    const REQUIREMENT: PermissionRequirement;
}

/// A set of stable specific-permission bits. Unknown bits round-trip but cannot be
/// requested by a policy. This small value needs no serialization or operator API.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SpecificPermissions(u32);

impl SpecificPermissions {
    pub const EMPTY: Self = Self(0);

    pub const fn from_bits_retain(bits: u32) -> Self {
        Self(bits)
    }

    pub const fn bits(self) -> u32 {
        self.0
    }

    pub const fn contains(self, permission: SpecificPermission) -> bool {
        self.0 & permission as u32 != 0
    }

    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

impl From<SpecificPermission> for SpecificPermissions {
    fn from(value: SpecificPermission) -> Self {
        Self(value as u32)
    }
}

/// Effective permissions for an already-scoped resource. Administrator access is
/// explicit, never represented by an invalid hierarchy value or partial bit mask.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectivePermission {
    Administrator,
    Granted {
        level: PermissionLevel,
        specifics: SpecificPermissions,
    },
}

impl EffectivePermission {
    pub const fn allows(self, requirement: PermissionRequirement) -> bool {
        match self {
            Self::Administrator => true,
            Self::Granted { level, specifics } => {
                level.grants(requirement.level)
                    && match requirement.specific {
                        Some(permission) => specifics.contains(permission),
                        None => true,
                    }
            }
        }
    }
}

#[macro_export]
macro_rules! permission_policy {
    ($name:ident, $resource:expr, $level:expr $(, $specific:expr)?) => {
        pub struct $name;
        impl $crate::PermissionPolicy for $name {
            const REQUIREMENT: $crate::PermissionRequirement = $crate::PermissionRequirement {
                resource_type: $resource,
                level: $level,
                specific: $crate::permission_policy!(@specific $($specific)?),
            };
        }
    };
    (@specific $specific:expr) => { Some($specific) };
    (@specific) => { None };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sql_accepted_levels_match_every_hierarchy_pair() {
        let levels = [
            PermissionLevel::None,
            PermissionLevel::Read,
            PermissionLevel::Write,
            PermissionLevel::Execute,
        ];
        for required in levels {
            for granted in levels {
                assert_eq!(
                    granted.grants(required),
                    required
                        .accepted_database_levels()
                        .contains(&(granted as i32))
                );
            }
        }
        for invalid in [-1, 3, 5, 6, 7, 8, i32::MAX] {
            assert_eq!(PermissionLevel::from_i32(invalid), None);
            assert!(
                levels
                    .iter()
                    .all(|level| !level.accepted_database_levels().contains(&invalid))
            );
        }
    }

    #[test]
    fn specific_sets_preserve_unknown_bits_without_granting_known_permissions() {
        let unknown = SpecificPermissions::from_bits_retain(1 << 30);
        assert!(
            SpecificPermission::ALL
                .into_iter()
                .all(|bit| !unknown.contains(bit))
        );
        let combined = unknown.union(SpecificPermission::Apply.into());
        assert!(combined.contains(SpecificPermission::Apply));
        assert!(!combined.contains(SpecificPermission::Logs));
        assert_eq!(
            combined.bits(),
            (1 << 30) | SpecificPermission::Apply as u32
        );
        assert_eq!(
            SpecificPermissions::from_bits_retain(combined.bits()),
            combined
        );
    }

    #[test]
    fn administrators_need_no_synthetic_masks() {
        for specific in SpecificPermission::ALL {
            assert!(
                EffectivePermission::Administrator.allows(PermissionRequirement {
                    resource_type: ResourceType::Deployment,
                    level: PermissionLevel::Execute,
                    specific: Some(specific),
                })
            );
        }
    }
}
