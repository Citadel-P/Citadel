use crate::IdentityError;

pub const MINIMUM_PASSWORD_CHARACTERS: usize = 15;
pub const MAXIMUM_PASSWORD_CHARACTERS: usize = 128;

/// Applied only when setting passwords, never when verifying existing credentials.
#[derive(Debug, Clone, Copy)]
pub struct PasswordPolicy {
    minimum_length: usize,
}

impl Default for PasswordPolicy {
    fn default() -> Self {
        Self {
            minimum_length: MINIMUM_PASSWORD_CHARACTERS,
        }
    }
}

impl PasswordPolicy {
    pub fn new(minimum_length: usize) -> Result<Self, IdentityError> {
        if !(8..=MAXIMUM_PASSWORD_CHARACTERS).contains(&minimum_length) {
            return Err(IdentityError::Validation(
                "Password minimum length must be between 8 and 128 characters.".into(),
            ));
        }
        Ok(Self { minimum_length })
    }

    pub fn minimum_length(self) -> usize {
        self.minimum_length
    }

    pub fn validate(
        &self,
        password: &str,
        user_name: Option<&str>,
        email: Option<&str>,
    ) -> Result<(), IdentityError> {
        let count = password.chars().count();
        if !(self.minimum_length..=MAXIMUM_PASSWORD_CHARACTERS).contains(&count) {
            return Err(IdentityError::Validation(format!(
                "Password must contain {} to {MAXIMUM_PASSWORD_CHARACTERS} characters.",
                self.minimum_length
            )));
        }
        const BLOCKED: [&str; 7] = [
            "admin",
            "admin123",
            "password",
            "password123",
            "letmein",
            "citadel",
            "citadel123",
        ];
        if BLOCKED
            .iter()
            .any(|blocked| password.eq_ignore_ascii_case(blocked))
            || include_str!("common-passwords.txt")
                .lines()
                .any(|blocked| password.eq_ignore_ascii_case(blocked))
            || user_name.is_some_and(|name| password.eq_ignore_ascii_case(name))
            || email.is_some_and(|address| {
                password.eq_ignore_ascii_case(address)
                    || address
                        .split_once('@')
                        .is_some_and(|(local, _)| password.eq_ignore_ascii_case(local))
            })
        {
            return Err(IdentityError::Validation(
                "Choose a less predictable password.".to_owned(),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn configurable_length_preserves_unicode_passphrases_and_weak_password_checks() {
        let policy = PasswordPolicy::new(8).unwrap();
        assert!(policy.validate("river oak", None, None).is_ok());
        assert!(
            PasswordPolicy::default()
                .validate("river oak", None, None)
                .is_err()
        );
        assert!(policy.validate("password", None, None).is_err());
        assert!(policy.validate("12345678", None, None).is_err());
        assert!(
            policy
                .validate("owner@example.test", None, Some("owner@example.test"))
                .is_err()
        );
        assert!(policy.validate(&"🌲".repeat(8), None, None).is_ok());
        assert!(policy.validate(&"🌲".repeat(7), None, None).is_err());
        assert!(policy.validate(&"x".repeat(128), None, None).is_ok());
        assert!(policy.validate(&"x".repeat(129), None, None).is_err());
        for length in [0, 7, 129] {
            assert!(PasswordPolicy::new(length).is_err());
        }
    }
}
