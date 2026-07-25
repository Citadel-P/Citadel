using Application.TaskJobs;

namespace Tests.Unit.Application.TaskJobs;

public sealed class LicenseTransitionMonitorJobTests
{
    private static readonly DateTimeOffset Now = new(2026, 7, 25, 12, 0, 0, TimeSpan.Zero);

    [Fact]
    public void CalculateNextCheckDelay_WithoutBoundary_ReturnsMaximumInterval()
    {
        var delay = LicenseTransitionMonitorJob.CalculateNextCheckDelay(Now, null);

        Assert.Equal(TimeSpan.FromHours(1), delay);
    }

    [Fact]
    public void CalculateNextCheckDelay_WithDistantBoundary_ReturnsMaximumInterval()
    {
        var delay = LicenseTransitionMonitorJob.CalculateNextCheckDelay(Now, Now.AddHours(2));

        Assert.Equal(TimeSpan.FromHours(1), delay);
    }

    [Fact]
    public void CalculateNextCheckDelay_WithUpcomingBoundary_IncludesMargin()
    {
        var delay = LicenseTransitionMonitorJob.CalculateNextCheckDelay(Now, Now.AddMinutes(5));

        Assert.Equal(TimeSpan.FromMinutes(5) + TimeSpan.FromSeconds(1), delay);
    }

    [Theory]
    [InlineData(0)]
    [InlineData(-1)]
    public void CalculateNextCheckDelay_AtOrPastBoundary_ReturnsMargin(int offsetSeconds)
    {
        var delay = LicenseTransitionMonitorJob.CalculateNextCheckDelay(
            Now,
            Now.AddSeconds(offsetSeconds));

        Assert.Equal(TimeSpan.FromSeconds(1), delay);
    }
}
