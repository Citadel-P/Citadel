use citadel_identity::{UserDateTimeFormat, UserTheme};
use citadel_primitives::ResourceType;

#[test]
fn resource_values_match_the_accepted_database_contract() {
    assert_eq!(ResourceType::Platform as i32, 0);
    assert_eq!(ResourceType::ServiceAccount as i32, 21);
    for (index, resource) in ResourceType::ALL.into_iter().enumerate() {
        assert_eq!(ResourceType::from_i32(index as i32), Some(resource));
    }
    assert_eq!(ResourceType::from_i32(22), None);
}

#[test]
fn preference_values_match_the_existing_database_contract() {
    assert_eq!(
        UserDateTimeFormat::from_database_str("TwentyFourHour"),
        Some(UserDateTimeFormat::TwentyFourHour)
    );
    assert_eq!(UserTheme::Dark.as_database_str(), "Dark");
    assert_eq!(UserTheme::from_database_str("dark"), None);
}
