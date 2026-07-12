using Domain;
using Domain.Contracts.Resources.Identity;
using System.Text.Json.Serialization;

namespace WebApi.Routes.Endpoints.Resources.Identity.Profile;

public sealed record UserPreferencesView(
    [property: JsonIgnore(Condition = JsonIgnoreCondition.Never)]
    string? TimeZone,
    UserDateTimeFormat DateTimeFormat,
    UserTheme Theme,
    bool IsPersisted)
{
    internal static UserPreferencesView Map(UserPreferencesDetails preferences)
        => new(
            preferences.TimeZone,
            preferences.DateTimeFormat,
            preferences.Theme,
            preferences.IsPersisted);
}
