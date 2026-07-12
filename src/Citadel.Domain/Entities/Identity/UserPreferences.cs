namespace Domain.Entities.Identity;

public sealed class UserPreferences
{
    public Guid UserId { get; private set; }
    public string TimeZone { get; private set; } = null!;
    public UserDateTimeFormat DateTimeFormat { get; private set; }
    public UserTheme Theme { get; private set; }
    public DateTime UpdatedAt { get; private set; }

    public User User { get; private set; } = null!;

    public static UserPreferences Create(
        Guid userId,
        string timeZone,
        UserDateTimeFormat dateTimeFormat,
        UserTheme theme,
        DateTime updatedAt)
        => new()
        {
            UserId = userId,
            TimeZone = timeZone,
            DateTimeFormat = dateTimeFormat,
            Theme = theme,
            UpdatedAt = updatedAt,
        };

    public void Update(
        string timeZone,
        UserDateTimeFormat dateTimeFormat,
        UserTheme theme,
        DateTime updatedAt)
    {
        TimeZone = timeZone;
        DateTimeFormat = dateTimeFormat;
        Theme = theme;
        UpdatedAt = updatedAt;
    }
}
