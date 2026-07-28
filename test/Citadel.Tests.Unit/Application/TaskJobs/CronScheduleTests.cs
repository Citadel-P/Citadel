using Application.TaskJobs;

namespace Tests.Unit.Application.TaskJobs;

public sealed class CronScheduleTests
{
    [Theory]
    [InlineData("0 9 1 * 1", "2026-07-01T09:00:00Z")]
    [InlineData("0 9 1 * 1", "2026-06-08T09:00:00Z")]
    public void IsDue_WhenDayOfMonthAndDayOfWeekAreRestricted_ShouldMatchEither(
        string expression,
        string utcTimestamp)
    {
        var now = DateTime.Parse(
            utcTimestamp,
            null,
            System.Globalization.DateTimeStyles.AdjustToUniversal);

        Assert.True(CronSchedule.IsDue(expression, "UTC", now));
    }

    [Fact]
    public void IsDue_WhenNeitherRestrictedDayMatches_ShouldReturnFalse()
    {
        var now = new DateTime(2026, 6, 9, 9, 0, 0, DateTimeKind.Utc);

        Assert.False(CronSchedule.IsDue("0 9 1 * 1", "UTC", now));
    }

    [Theory]
    [InlineData("0 9 * * 1", "2026-06-08T09:00:00Z", true)]
    [InlineData("0 9 * * 1", "2026-06-09T09:00:00Z", false)]
    [InlineData("0 9 8 * *", "2026-06-08T09:00:00Z", true)]
    [InlineData("0 9 8 * *", "2026-06-09T09:00:00Z", false)]
    public void IsDue_WhenOneDayFieldIsWildcard_ShouldRequireTheRestrictedField(
        string expression,
        string utcTimestamp,
        bool expected)
    {
        var now = DateTime.Parse(
            utcTimestamp,
            null,
            System.Globalization.DateTimeStyles.AdjustToUniversal);

        Assert.Equal(expected, CronSchedule.IsDue(expression, "UTC", now));
    }
}
