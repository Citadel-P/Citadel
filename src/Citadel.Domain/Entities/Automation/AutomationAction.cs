using Domain.Contracts.Resources;
using Domain.Entities;
using Domain.Entities.Tags;

namespace Domain.Entities.Automation;

public sealed class AutomationAction(
    string name,
    string? description,
    string code,
    string defaultArgsJson,
    bool enabled,
    bool scheduleEnabled,
    string? scheduleCron,
    string scheduleTimeZone,
    AutomationWebhookConfig? webhook,
    int timeoutSeconds,
    bool alertOnFailure,
    Guid runAsActorId,
    Guid createdByActorId,
    DateTime? lastScheduledRunAt = null,
    ResourceControlState controlState = ResourceControlState.Idle,
    Guid? currentRunId = null,
    long? controlStartedAt = null,
    long rowVersion = 0,
    DateTime? createdAt = null,
    DateTime? updatedAt = null) : IAuditedEntity, IReconcilableResource
{
    public Guid Id { get; private set; } = Guid.CreateVersion7();
    public string Name { get; private set; } = NormalizeRequired(name);
    public string? Description { get; private set; } = NormalizeOptional(description);
    public string Code { get; private set; } = code;
    public string DefaultArgsJson { get; private set; } = NormalizeJson(defaultArgsJson);
    public bool Enabled { get; private set; } = enabled;
    public bool ScheduleEnabled { get; private set; } = scheduleEnabled;
    public string? ScheduleCron { get; private set; } = NormalizeOptional(scheduleCron);
    public string ScheduleTimeZone { get; private set; } = NormalizeRequired(scheduleTimeZone);
    public AutomationWebhookConfig? Webhook { get; private set; } = NormalizeWebhook(webhook);
    public bool WebhookEnabled => Webhook?.Enabled == true;
    public int TimeoutSeconds { get; private set; } = timeoutSeconds;
    public bool AlertOnFailure { get; private set; } = alertOnFailure;
    public Guid RunAsActorId { get; private set; } = runAsActorId;
    public DateTime? LastScheduledRunAt { get; private set; } = lastScheduledRunAt;
    public ResourceControlState ControlState { get; private set; } = controlState;
    public Guid? CurrentRunId { get; private set; } = currentRunId;
    public long? ControlStartedAt { get; private set; } = controlStartedAt;
    public long RowVersion { get; private set; } = rowVersion;
    public Guid? ControlTriggeredBy => null;
    public DateTime CreatedAt { get; private set; } = createdAt ?? DateTime.UtcNow;
    public Guid CreatedByActorId { get; private set; } = createdByActorId;
    public DateTime UpdatedAt { get; private set; } = updatedAt ?? DateTime.UtcNow;
    public IReadOnlyList<TagSummary> Tags { get; private set; } = [];

    public void Rename(string name)
    {
        Name = NormalizeRequired(name);
        Touch();
    }

    public void UpdateDescription(string? description)
    {
        Description = NormalizeOptional(description);
        Touch();
    }

    public void AssignTags(IReadOnlyList<TagSummary> tags)
    {
        Tags = tags;
    }

    public void Update(
        string? description,
        bool updateDescription,
        string? code,
        string? defaultArgsJson,
        bool? enabled,
        bool? scheduleEnabled,
        string? scheduleCron,
        bool updateScheduleCron,
        string? scheduleTimeZone,
        AutomationWebhookConfig? webhook,
        bool updateWebhook,
        int? timeoutSeconds,
        bool? alertOnFailure,
        Guid? runAsActorId)
    {
        if (updateDescription)
            Description = NormalizeOptional(description);

        if (code is not null)
            Code = code;

        if (defaultArgsJson is not null)
            DefaultArgsJson = NormalizeJson(defaultArgsJson);

        if (enabled.HasValue)
            Enabled = enabled.Value;

        if (scheduleEnabled.HasValue)
            ScheduleEnabled = scheduleEnabled.Value;

        if (updateScheduleCron)
            ScheduleCron = NormalizeOptional(scheduleCron);

        if (scheduleTimeZone is not null)
            ScheduleTimeZone = NormalizeRequired(scheduleTimeZone);

        if (updateWebhook)
            Webhook = NormalizeWebhook(webhook);

        if (timeoutSeconds.HasValue)
            TimeoutSeconds = timeoutSeconds.Value;

        if (alertOnFailure.HasValue)
            AlertOnFailure = alertOnFailure.Value;

        if (runAsActorId.HasValue && runAsActorId.Value != Guid.Empty)
            RunAsActorId = runAsActorId.Value;

        Touch();
    }

    public void MarkProcessing(Guid runId)
    {
        ControlState = ResourceControlState.Processing;
        CurrentRunId = runId;
        ControlStartedAt = DateTimeOffset.UtcNow.ToUnixTimeSeconds();
        Touch();
    }

    public void MarkIdle()
    {
        ControlState = ResourceControlState.Idle;
        CurrentRunId = null;
        ControlStartedAt = null;
        Touch();
    }

    public void MarkScheduled(DateTime scheduledMinuteUtc)
    {
        LastScheduledRunAt = scheduledMinuteUtc;
        Touch();
    }

    public void Validate()
    {
        if (string.IsNullOrWhiteSpace(Name))
            throw new ArgumentException("Automation action name is required.", nameof(Name));

        if (Name.Length > 128)
            throw new ArgumentException("Automation action name cannot exceed 128 characters.", nameof(Name));

        if (Description?.Length > 600)
            throw new ArgumentException("Automation action description cannot exceed 600 characters.", nameof(Description));

        if (string.IsNullOrWhiteSpace(Code))
            throw new ArgumentException("Automation action code is required.", nameof(Code));

        if (Code.Length > 262_144)
            throw new ArgumentException("Automation action code cannot exceed 256 KB.", nameof(Code));

        if (DefaultArgsJson.Length > 65_536)
            throw new ArgumentException("Automation action default args cannot exceed 64 KB.", nameof(DefaultArgsJson));

        if (ScheduleEnabled && string.IsNullOrWhiteSpace(ScheduleCron))
            throw new ArgumentException("Schedule cron is required when the schedule is enabled.", nameof(ScheduleCron));

        if (ScheduleCron?.Length > 128)
            throw new ArgumentException("Schedule cron cannot exceed 128 characters.", nameof(ScheduleCron));

        if (ScheduleTimeZone.Length > 128)
            throw new ArgumentException("Schedule time zone cannot exceed 128 characters.", nameof(ScheduleTimeZone));

        if (TimeoutSeconds is < 1 or > 86_400)
            throw new ArgumentException("Automation action timeout must be between 1 and 86400 seconds.", nameof(TimeoutSeconds));

        if (RunAsActorId == Guid.Empty)
            throw new ArgumentException("Run-as actor is required.", nameof(RunAsActorId));
    }

    public static AutomationAction FromPersistence(
        Guid id,
        string name,
        string? description,
        string code,
        string defaultArgsJson,
        bool enabled,
        bool scheduleEnabled,
        string? scheduleCron,
        string scheduleTimeZone,
        AutomationWebhookConfig? webhook,
        int timeoutSeconds,
        bool alertOnFailure,
        Guid runAsActorId,
        DateTime? lastScheduledRunAt,
        ResourceControlState controlState,
        Guid? currentRunId,
        long? controlStartedAt,
        long rowVersion,
        Guid createdByActorId,
        DateTime createdAt,
        DateTime updatedAt)
    {
        return new AutomationAction(
            name,
            description,
            code,
            defaultArgsJson,
            enabled,
            scheduleEnabled,
            scheduleCron,
            scheduleTimeZone,
            webhook,
            timeoutSeconds,
            alertOnFailure,
            runAsActorId,
            createdByActorId,
            lastScheduledRunAt,
            controlState,
            currentRunId,
            controlStartedAt,
            rowVersion,
            createdAt,
            updatedAt)
        {
            Id = id
        };
    }

    private void Touch()
    {
        UpdatedAt = DateTime.UtcNow;
        RowVersion++;
    }

    private static string NormalizeRequired(string value) => value.Trim();

    private static string? NormalizeOptional(string? value)
        => string.IsNullOrWhiteSpace(value) ? null : value.Trim();

    private static AutomationWebhookConfig? NormalizeWebhook(AutomationWebhookConfig? value)
        => value is null
            ? null
            : value with
            {
                Secret = NormalizeOptional(value.Secret),
                BranchFilter = NormalizeOptional(value.BranchFilter)
            };

    private static string NormalizeJson(string value)
        => string.IsNullOrWhiteSpace(value) ? "{}" : value.Trim();
}

public sealed record AutomationWebhookConfig(
    bool Enabled = false,
    WebhookProvider Provider = WebhookProvider.GitHub,
    WebhookAuthScheme AuthScheme = WebhookAuthScheme.GitHubHmacSha256,
    string? Secret = null,
    string? BranchFilter = null) : WebhookConfig(Enabled, Provider, AuthScheme, Secret, BranchFilter);
