using System.Diagnostics.CodeAnalysis;
using System.Text.Json.Serialization;

namespace Domain.Entities;

public class AlertRule : IAuditedEntity
{
    private readonly List<AlertRuleLimitedTo> _limitedTo = [];
    private readonly List<AlertRuleQuietHour> _quietHours = [];

    public Guid Id { get; private set; } = Guid.CreateVersion7();
    public string Url { get; private set; }
    public AlertType Type { get; private set; }
    public int CooldownSeconds { get; private set; }
    public bool IsEnabled { get; private set; }
    public AlertScope Scope { get; private set; }

    public IReadOnlyCollection<AlertRuleLimitedTo> LimitedTo => _limitedTo;
    public IReadOnlyCollection<AlertRuleQuietHour> QuietHours => _quietHours;

    #region IAuditedEntity
    public Guid CreatedByActorId { get; private set; }
    public DateTime CreatedAt { get; private set; }
    #endregion

    public AlertRule(
        string url,
        AlertType type,
        int cooldownSeconds,
        bool isEnabled,
        AlertScope scope,
        IEnumerable<AlertRuleLimitedTo>? limitedTo,
        IEnumerable<AlertRuleQuietHour>? quietHours,
        Guid createdByActorId)
    {
        if (string.IsNullOrWhiteSpace(url))
            throw new ArgumentException("URL is required", nameof(url));

        if (cooldownSeconds < 10 || cooldownSeconds > 86400)
            throw new ArgumentOutOfRangeException(nameof(cooldownSeconds), "Cooldown must be between 10s and 24h.");

        Url = url;
        Type = type;
        CooldownSeconds = cooldownSeconds;
        IsEnabled = isEnabled;
        Scope = scope;

        _limitedTo = limitedTo?.ToList() ?? [];
        _quietHours = quietHours?.ToList() ?? [];

        CreatedByActorId = createdByActorId;
        CreatedAt = DateTime.UtcNow;

        ValidateScope();
        ValidateResourceCompatibility();
        ValidateQuietHours();
    }

    public bool CanTrigger(DateTime utcNow, AlertRuleState? state)
    {
        if (!IsEnabled)
            return false;

        if (state is not null &&
            (utcNow - state.LastTriggeredAt).TotalSeconds < CooldownSeconds)
            return false;

        if (IsInQuietHours(utcNow))
            return false;

        return true;
    }

    private void ValidateScope()
    {
        if (Scope == AlertScope.All && _limitedTo.Any())
            throw new InvalidOperationException("LimitedTo must be empty when scope is All.");

        if (Scope == AlertScope.Specific && !_limitedTo.Any())
            throw new InvalidOperationException("LimitedTo must contain at least one resource when scope is Specific.");
    }

    private void ValidateResourceCompatibility()
    {
        var expected = Type switch
        {
            AlertType.PlatformCpuHigh or AlertType.PlatformRamHigh or AlertType.PlatformVersionMismatch
                => AlertResourceType.Platform,

            AlertType.DeploymentImageUpdateAvailable or AlertType.DeploymentAutoUpdated or AlertType.DeploymentFailed
                => AlertResourceType.Deployment,

            _ => AlertResourceType.Stack
        };

        if (Scope == AlertScope.Specific &&
            _limitedTo.Any(x => x.ResourceType != expected))
            throw new InvalidOperationException("ResourceType does not match AlertType.");
    }

    private bool IsInQuietHours(DateTime utcNow)
    {
        foreach (var quietHour in _quietHours)
        {
            if (quietHour.ScheduleType == ScheduleType.Daily)
            {
                if (IsInTimeRange(utcNow, quietHour.StartTime, quietHour.EndTime, quietHour.Timezone))
                    return true;
            }
            else if (quietHour is WeeklyQuietHour weekly)
            {
                if (utcNow.DayOfWeek == weekly.DayOfWeek &&
                    IsInTimeRange(utcNow, quietHour.StartTime, quietHour.EndTime, quietHour.Timezone))
                    return true;
            }
        }

        return false;
    }

    private void ValidateQuietHours()
    {
        for (int i = 0; i < _quietHours.Count; i++)
            for (int j = i + 1; j < _quietHours.Count; j++)
                if (Overlaps(_quietHours[i], _quietHours[j]))
                    throw new InvalidOperationException("Quiet hours overlap.");
    }

    private static bool Overlaps(AlertRuleQuietHour first, AlertRuleQuietHour second)
    {
        if (!string.Equals(first.Timezone, second.Timezone, StringComparison.OrdinalIgnoreCase))
            return false;

        if (first is WeeklyQuietHour fw && second is WeeklyQuietHour sw &&
            fw.DayOfWeek != sw.DayOfWeek)
            return false;

        foreach (var r1 in GetTimeRanges(first.StartTime, first.EndTime))
            foreach (var r2 in GetTimeRanges(second.StartTime, second.EndTime))
                if (r1.Start <= r2.End && r2.Start <= r1.End)
                    return true;

        return false;
    }

    private static IEnumerable<(TimeSpan Start, TimeSpan End)> GetTimeRanges(TimeOnly startTime, TimeOnly endTime)
    {
        var start = startTime.ToTimeSpan();
        var end = endTime.ToTimeSpan();

        if (startTime <= endTime)
        {
            yield return (start, end);
            yield break;
        }

        yield return (start, TimeSpan.FromDays(1));
        yield return (TimeSpan.Zero, end);
    }

    private static bool IsInTimeRange(DateTime utcNow, TimeOnly startTime, TimeOnly endTime, string timezone)
    {
        var tz = TimeZoneInfo.FindSystemTimeZoneById(timezone);
        var localNow = TimeOnly.FromTimeSpan(TimeZoneInfo.ConvertTime(utcNow, tz).TimeOfDay);

        return startTime <= endTime
            ? localNow >= startTime && localNow <= endTime
            : localNow >= startTime || localNow <= endTime;
    }
}

public sealed record AlertRuleLimitedTo(AlertResourceType ResourceType, Guid ResourceId);

[JsonPolymorphic]
[DynamicallyAccessedMembers(DynamicallyAccessedMemberTypes.All)]
[JsonDerivedType(typeof(DailyQuietHour), nameof(ScheduleType.Daily))]
[JsonDerivedType(typeof(WeeklyQuietHour), nameof(ScheduleType.Weekly))]
public record AlertRuleQuietHour(string Name, ScheduleType ScheduleType, TimeOnly StartTime, TimeOnly EndTime, string Timezone, string? Description);

public sealed record DailyQuietHour(string Name, TimeOnly StartTime, TimeOnly EndTime, string Timezone, string? Description)
    : AlertRuleQuietHour(Name, ScheduleType.Daily, StartTime, EndTime, Timezone, Description);

public sealed record WeeklyQuietHour(string Name, DayOfWeek DayOfWeek, TimeOnly StartTime, TimeOnly EndTime, string Timezone, string? Description)
    : AlertRuleQuietHour(Name, ScheduleType.Weekly, StartTime, EndTime, Timezone, Description);


public class AlertRuleState
{
    public Guid AlertRuleId { get; private set; }
    public Guid ResourceId { get; private set; }

    public DateTime LastTriggeredAt { get; private set; }

    private AlertRuleState() { }

    public AlertRuleState(Guid alertRuleId, Guid resourceId)
    {
        AlertRuleId = alertRuleId;
        ResourceId = resourceId;
        LastTriggeredAt = DateTime.MinValue;
    }

    public void MarkTriggered(DateTime utcNow)
    {
        LastTriggeredAt = utcNow;
    }
}