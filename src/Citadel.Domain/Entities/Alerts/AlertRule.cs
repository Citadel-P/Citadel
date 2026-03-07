using System.Diagnostics.CodeAnalysis;
using System.Text.Json.Serialization;

namespace Domain.Entities.Alerts;

public sealed class AlertRule : IAuditedEntity
{
    private readonly List<Guid> _channelIds = [];
    private readonly List<AlertRuleLimitedTo> _limitedTo = [];
    private readonly List<AlertRuleQuietHour> _quietHours = [];

    public Guid Id { get; private set; } = Guid.CreateVersion7();
    public string Name { get; private set; }
    public AlertType Type { get; private set; }
    public AlertSeverity Severity { get; private set; }
    /// <summary>
    /// Gets the cooldown period, in seconds, before the alert can be raised again.
    /// </summary>
    public int? CooldownSeconds { get; private set; }
    /// <summary>
    /// Gets the number of matches required to satisfy the condition, only meaningful for threshold rules: CpuHigh, RamHigh, VersionMismatch.
    /// For example, if RequiredMatches is 3 for a CpuHigh rule, the alert will only be triggered if the CPU usage is high for 3 consecutive checks.  
    /// </summary>
    public int? RequiredMatches { get; private set; }
    /// <summary>
    /// Gets the threshold value used to determine whether a specific condition is met.
    /// </summary>
    public double? Threshold { get; private set; }
    public AlertRuleStatus Status { get; private set; }

    public IReadOnlyCollection<Guid> ChannelIds => _channelIds;
    public IReadOnlyCollection<AlertRuleLimitedTo> LimitedTo => _limitedTo;
    public IReadOnlyCollection<AlertRuleQuietHour> QuietHours => _quietHours;

    #region IAuditedEntity
    public Guid CreatedByActorId { get; private set; }
    public DateTime CreatedAt { get; private set; }
    #endregion

    [JsonConstructor]
    public AlertRule(
        string? name,
        AlertType type,
        AlertSeverity severity,
        int? cooldownSeconds,
        AlertRuleStatus status,
        Guid createdByActorId,
        int? requiredMatches = null,
        double? threshold = null,
        IReadOnlyCollection<Guid>? channelIds = null,
        IReadOnlyCollection<AlertRuleLimitedTo>? limitedTo = null,
        IReadOnlyCollection<AlertRuleQuietHour>? quietHours = null)
    {
        if (cooldownSeconds < 10 || cooldownSeconds > 86400)
            throw new ArgumentOutOfRangeException(nameof(cooldownSeconds), "Cooldown must be between 10s and 24h.");

        Name = string.IsNullOrWhiteSpace(name) ? type.ToString() : name;
        Type = type;
        Severity = severity;
        CooldownSeconds = cooldownSeconds;
        RequiredMatches = requiredMatches;
        Threshold = threshold;
        Status = status;

        _channelIds = channelIds?.ToList() ?? [];
        _limitedTo = limitedTo?.ToList() ?? [];
        _quietHours = quietHours?.ToList() ?? [];

        CreatedByActorId = createdByActorId;
        CreatedAt = DateTime.UtcNow;

        ValidateThresholdConfiguration();
        ValidateResourceCompatibility();
        ValidateQuietHours();
    }

    public static AlertRule FromPersistence(
        Guid id,
        string? name,
        AlertType type,
        AlertSeverity severity,
        int? cooldownSeconds,
        AlertRuleStatus status,
        Guid createdByActorId,
        DateTime createdAt,
        int? requiredMatches = null,
        double? threshold = null,
        IEnumerable<Guid>? channelIds = null,
        IEnumerable<AlertRuleLimitedTo>? limitedTo = null,
        IEnumerable<AlertRuleQuietHour>? quietHours = null)
    {
        var rule = new AlertRule(
            name,
            type,
            severity,
            cooldownSeconds,
            status,
            createdByActorId,
            requiredMatches,
            threshold,
            channelIds?.ToList(),
            limitedTo?.ToList(),
            quietHours?.ToList())
        {
            Id = id,
            CreatedAt = createdAt,
            CreatedByActorId = createdByActorId,
        };
       
        return rule;
    }

    public AlertRule Disable()
    {
        Status = AlertRuleStatus.Disabled;
        return this;
    }

    public void PartialUpdate(
        string name,
        AlertType type,
        AlertSeverity severity,
        int? cooldownSeconds,
        AlertRuleStatus status,
        IEnumerable<Guid>? channelIds = null,
        int? requiredMatches = null,
        double? threshold = null,
        IEnumerable<AlertRuleLimitedTo>? limitedTo = null,
        IEnumerable<AlertRuleQuietHour>? quietHours = null)
    {
        Name = name;
        Type = type;
        Severity = severity;
        CooldownSeconds = cooldownSeconds;
        Status = status;
        _channelIds.Clear();
        if (channelIds is not null) _channelIds.AddRange(channelIds);
        RequiredMatches = requiredMatches;
        Threshold = threshold;
        _limitedTo.Clear();
        if (limitedTo is not null) _limitedTo.AddRange(limitedTo);
        _quietHours.Clear();
        if (quietHours is not null) _quietHours.AddRange(quietHours);
    }

    public bool CanTrigger(DateTime utcNow, AlertRuleState? state)
    {
        if (Status is AlertRuleStatus.Disabled)
            return false;

        if (state is not null &&
            (utcNow - state.LastTriggeredAt)?.TotalSeconds < CooldownSeconds)
            return false;

        if (_quietHours.Any(q => q.IsInQuietHours(utcNow)))
            return false;

        return true;
    }

    private void ValidateThresholdConfiguration()
    {
        if (AlertTypeMetadata.IsThreshold(Type))
        {
            if (RequiredMatches is null || Threshold is null)
                throw new InvalidOperationException($"{nameof(Threshold)} alerts require {nameof(RequiredMatches)} and {nameof(Threshold)}.");
        }
        else
        {
            if (RequiredMatches is not null || Threshold is not null)
                throw new InvalidOperationException($"Non-threshold alerts must not define {nameof(RequiredMatches)} or {nameof(Threshold)}.");
        }
    }

    private void ValidateResourceCompatibility()
    {
        var expected = AlertTypeMetadata.GetResourceType(Type);

        if (_limitedTo.Any(x => x.ResourceType != expected))
            throw new InvalidOperationException("ResourceType does not match AlertType.");
    }

    private void ValidateQuietHours()
    {
        for (int i = 0; i < _quietHours.Count; i++)
            for (int j = i + 1; j < _quietHours.Count; j++)
                if (_quietHours[i].Overlaps(_quietHours[j]))
                    throw new InvalidOperationException("Quiet hours overlap.");
    }
}
[JsonPolymorphic]
[DynamicallyAccessedMembers(DynamicallyAccessedMemberTypes.All)]
[JsonDerivedType(typeof(DailyQuietHour), nameof(ScheduleType.Daily))]
[JsonDerivedType(typeof(WeeklyQuietHour), nameof(ScheduleType.Weekly))]
public abstract record AlertRuleQuietHour(
    string Name,
    ScheduleType ScheduleType,
    TimeOnly StartTime,
    TimeOnly EndTime,
    string Timezone,
    string? Description)
{
    public TimeZoneInfo TimeZoneInfo { get; } =
        TimeZoneInfo.FindSystemTimeZoneById(Timezone);

    public bool IsInQuietHours(DateTime utcNow)
    {
        var localTime = TimeOnly.FromTimeSpan(
            TimeZoneInfo.ConvertTimeFromUtc(utcNow, TimeZoneInfo).TimeOfDay);

        return IsInRange(localTime) && MatchesDay(utcNow, TimeZoneInfo);
    }

    protected abstract bool MatchesDay(DateTime utcNow, TimeZoneInfo tz);

    protected bool IsInRange(TimeOnly localTime)
        => StartTime <= EndTime
            ? localTime >= StartTime && localTime <= EndTime
            : localTime >= StartTime || localTime <= EndTime;

    public bool Overlaps(AlertRuleQuietHour other)
    {
        if (!string.Equals(Timezone, other.Timezone, StringComparison.OrdinalIgnoreCase))
            return false;

        if (!DayMatches(other))
            return false;

        foreach (var r1 in GetRanges())
            foreach (var r2 in other.GetRanges())
                if (r1.Start <= r2.End && r2.Start <= r1.End)
                    return true;

        return false;
    }

    protected abstract bool DayMatches(AlertRuleQuietHour other);

    protected IEnumerable<(TimeSpan Start, TimeSpan End)> GetRanges()
    {
        var start = StartTime.ToTimeSpan();
        var end = EndTime.ToTimeSpan();

        if (StartTime <= EndTime)
            yield return (start, end);
        else
        {
            yield return (start, TimeSpan.FromDays(1));
            yield return (TimeSpan.Zero, end);
        }
    }
}

public sealed record DailyQuietHour(
    string Name,
    TimeOnly StartTime,
    TimeOnly EndTime,
    string Timezone,
    string? Description)
    : AlertRuleQuietHour(Name, ScheduleType.Daily, StartTime, EndTime, Timezone, Description)
{
    protected override bool MatchesDay(DateTime utcNow, TimeZoneInfo tz) => true;
    protected override bool DayMatches(AlertRuleQuietHour other) => true;
}

public sealed record WeeklyQuietHour(
    string Name,
    DayOfWeek DayOfWeek,
    TimeOnly StartTime,
    TimeOnly EndTime,
    string Timezone,
    string? Description)
    : AlertRuleQuietHour(Name, ScheduleType.Weekly, StartTime, EndTime, Timezone, Description)
{
    protected override bool MatchesDay(DateTime utcNow, TimeZoneInfo tz)
        => TimeZoneInfo.ConvertTimeFromUtc(utcNow, tz).DayOfWeek == DayOfWeek;

    protected override bool DayMatches(AlertRuleQuietHour other)
        => other is WeeklyQuietHour w && w.DayOfWeek == DayOfWeek;
}

public sealed record AlertRuleLimitedTo(AlertResourceType ResourceType, Guid ResourceId);

public sealed class AlertRuleState : IAuditedEntity
{
    public Guid AlertRuleId { get; }
    public Guid ResourceId { get; }
    /// <summary>
    /// Gets the number of consecutive matches required to trigger the alert - only available for types that has threshold.
    /// </summary>
    public int ConsecutiveMatches { get; private set; }
    public DateTime? LastTriggeredAt { get; private set; }

    #region IAuditedEntity
    public Guid CreatedByActorId { get; private set; }
    public DateTime CreatedAt { get; private set; }
    #endregion

    public AlertRuleState(Guid alertRuleId, Guid resourceId, Guid actorId, int consecutiveMatches)
    {
        AlertRuleId = alertRuleId;
        ResourceId = resourceId;
        CreatedByActorId = actorId;
        ConsecutiveMatches = consecutiveMatches;
        CreatedAt = DateTime.UtcNow;
        LastTriggeredAt = null;
    }

    public static AlertRuleState FromPersistence(
        Guid alertRuleId, 
        Guid resourceId, 
        int consecutiveMatches, 
        DateTime? lastTriggeredAt, 
        Guid createdByActorId, 
        DateTime createdAt)
    {
        return new AlertRuleState(alertRuleId, resourceId, createdByActorId, consecutiveMatches)
        {
            LastTriggeredAt = lastTriggeredAt,
            CreatedAt = createdAt
        };
    }

    public void RegisterMatch() => ConsecutiveMatches++;
    public void Reset() => ConsecutiveMatches = 0;

    public bool CanFire(AlertRule rule)
        => rule.RequiredMatches is null || ConsecutiveMatches >= rule.RequiredMatches;

    public void MarkTriggered(DateTime utcNow)
        => LastTriggeredAt = utcNow;
}

public static class AlertTypeMetadata
{
    private static readonly Dictionary<AlertType, AlertResourceType> ResourceMap = new()
    {
        { AlertType.PlatformCpuHigh, AlertResourceType.Platform },
        { AlertType.PlatformRamHigh, AlertResourceType.Platform },
        { AlertType.PlatformUnreachable, AlertResourceType.Platform },
        { AlertType.PlatformVersionMismatch, AlertResourceType.Platform },
        { AlertType.UnmanagedContainerCreated, AlertResourceType.Platform },

        { AlertType.DeploymentImageUpdateAvailable, AlertResourceType.Deployment },
        { AlertType.DeploymentAutoUpdated, AlertResourceType.Deployment },
        { AlertType.DeploymentAutoDeployFailed, AlertResourceType.Deployment },

        { AlertType.StackImageUpdateAvailable, AlertResourceType.Stack },
        { AlertType.StackAutoUpdated, AlertResourceType.Stack },
        { AlertType.StackAutoDeployFailed, AlertResourceType.Stack },
    };

    private static readonly HashSet<AlertType> ThresholdTypes =
    [
        AlertType.PlatformCpuHigh,
        AlertType.PlatformRamHigh,
    ];

    public static AlertResourceType GetResourceType(AlertType type)
        => ResourceMap[type];

    public static bool IsThreshold(AlertType type)
        => ThresholdTypes.Contains(type);

    public static bool IsValidInfo(AlertType type, AlertEventInfo info)
        => (type, info) switch
        {
            (AlertType.PlatformCpuHigh, PlatformCpuHighAlertInfo) => true,
            (AlertType.PlatformRamHigh, PlatformRamHighAlertInfo) => true,
            (AlertType.PlatformUnreachable, PlatformUnreachableAlertInfo) => true,
            (AlertType.PlatformVersionMismatch, PlatformVersionMismatchAlertInfo) => true,
            (AlertType.UnmanagedContainerCreated, UnmanagedContainerCreatedAlertInfo) => true,
            (AlertType.DeploymentImageUpdateAvailable, DeploymentImageUpdateAvailableAlertInfo) => true,
            (AlertType.DeploymentAutoUpdated, DeploymentAutoUpdatedAlertInfo) => true,
            (AlertType.DeploymentAutoDeployFailed, DeploymentFailedAlertInfo) => true,
            (AlertType.StackImageUpdateAvailable, StackImageUpdateAvailableAlertInfo) => true,
            (AlertType.StackAutoUpdated, StackAutoUpdatedAlertInfo) => true,
            (AlertType.StackAutoDeployFailed, StackDeployFailedAlertInfo) => true,
            _ => false
        };
}