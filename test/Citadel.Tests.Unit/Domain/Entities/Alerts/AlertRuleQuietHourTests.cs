using Domain.Entities.Alerts;
using System.Text.Json;

namespace Tests.Unit.Domain.Entities.Alerts;

public sealed class AlertRuleQuietHourTests
{
    [Theory]
    [InlineData(2026, 7, 14, 8, 30)]
    [InlineData(2026, 12, 14, 9, 30)]
    public void IsInQuietHours_ShouldApplyIanaTimeZoneAndDaylightSavingOffset(
        int year,
        int month,
        int day,
        int hour,
        int minute)
    {
        var quietHour = new DailyQuietHour(
            "Paris morning",
            new TimeOnly(10, 0),
            new TimeOnly(11, 0),
            "Europe/Paris",
            Description: null);
        var utcNow = new DateTime(year, month, day, hour, minute, 0, DateTimeKind.Utc);

        Assert.True(quietHour.IsInQuietHours(utcNow));
    }

    [Fact]
    public void IsInQuietHours_ShouldUsePreviousDayForWeeklyRangeCrossingMidnight()
    {
        var quietHour = new WeeklyQuietHour(
            "Sunday night",
            DayOfWeek.Sunday,
            new TimeOnly(22, 0),
            new TimeOnly(2, 0),
            "Europe/Paris",
            Description: null);
        var mondayAtOneInParis = new DateTime(2026, 7, 19, 23, 0, 0, DateTimeKind.Utc);

        Assert.True(quietHour.IsInQuietHours(mondayAtOneInParis));
    }

    [Fact]
    public void Constructor_ShouldRejectUnknownTimeZone()
    {
        Assert.Throws<TimeZoneNotFoundException>(() =>
            new DailyQuietHour(
                "Invalid",
                new TimeOnly(10, 0),
                new TimeOnly(11, 0),
                "Not/A_Timezone",
                Description: null));
    }

    [Fact]
    public void Overlaps_ShouldDetectWeeklyOverlapOnFollowingDay()
    {
        var sundayNight = new WeeklyQuietHour(
            "Sunday night",
            DayOfWeek.Sunday,
            new TimeOnly(22, 0),
            new TimeOnly(2, 0),
            "Europe/Paris",
            Description: null);
        var mondayMorning = new WeeklyQuietHour(
            "Monday morning",
            DayOfWeek.Monday,
            new TimeOnly(1, 0),
            new TimeOnly(3, 0),
            "Europe/Paris",
            Description: null);

        Assert.True(sundayNight.Overlaps(mondayMorning));
    }

    [Fact]
    public void Overlaps_ShouldNotTreatEarlyHoursAsPartOfSameDayOvernightRange()
    {
        var sundayNight = new WeeklyQuietHour(
            "Sunday night",
            DayOfWeek.Sunday,
            new TimeOnly(22, 0),
            new TimeOnly(2, 0),
            "Europe/Paris",
            Description: null);
        var sundayMorning = new WeeklyQuietHour(
            "Sunday morning",
            DayOfWeek.Sunday,
            new TimeOnly(1, 0),
            new TimeOnly(3, 0),
            "Europe/Paris",
            Description: null);

        Assert.False(sundayNight.Overlaps(sundayMorning));
    }

    [Fact]
    public void Overlaps_ShouldNormalizeEquivalentIanaAndWindowsTimeZoneIds()
    {
        var iana = new DailyQuietHour(
            "IANA",
            new TimeOnly(10, 0),
            new TimeOnly(11, 0),
            "Europe/Paris",
            Description: null);
        var windows = new DailyQuietHour(
            "Windows",
            new TimeOnly(10, 30),
            new TimeOnly(11, 30),
            "Romance Standard Time",
            Description: null);

        Assert.True(iana.Overlaps(windows));
    }

    [Fact]
    public void Serialization_ShouldExcludeComputedTimeZoneInfo()
    {
        var quietHour = new DailyQuietHour(
            "Paris morning",
            new TimeOnly(10, 0),
            new TimeOnly(11, 0),
            "Europe/Paris",
            Description: null);

        var json = JsonSerializer.Serialize(quietHour);

        Assert.DoesNotContain("timeZoneInfo", json, StringComparison.OrdinalIgnoreCase);
        Assert.Contains("Europe/Paris", json, StringComparison.Ordinal);
    }
}
