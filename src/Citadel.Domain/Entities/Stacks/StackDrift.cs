using System.Diagnostics.CodeAnalysis;
using System.Text.Json.Serialization;

namespace Domain.Entities.Stacks;

public sealed record StackDriftPolicy(
    StackDriftMode Mode,
    bool AlertOnDrift,
    bool MarkDegraded,
    bool AutoStartStoppedContainers,
    bool AutoResumePausedContainers,
    bool RemoveExtraContainers)
{
    public static StackDriftPolicy Disabled { get; } = new(
        Mode: StackDriftMode.Disabled,
        AlertOnDrift: false,
        MarkDegraded: false,
        AutoStartStoppedContainers: false,
        AutoResumePausedContainers: false,
        RemoveExtraContainers: false);

    public static StackDriftPolicy Default { get; } = new(
        Mode: StackDriftMode.DetectOnly,
        AlertOnDrift: true,
        MarkDegraded: true,
        AutoStartStoppedContainers: false,
        AutoResumePausedContainers: false,
        RemoveExtraContainers: false);

    public StackDriftPolicy Normalize()
        => Mode == StackDriftMode.Disabled
            ? this with
            {
                AlertOnDrift = false,
                MarkDegraded = false,
                AutoStartStoppedContainers = false,
                AutoResumePausedContainers = false,
                RemoveExtraContainers = false
            }
            : this;
}

public sealed record StackDriftReport(
    Guid StackId,
    Guid PlatformId,
    bool HasDrift,
    bool HasAutoFixableDrift,
    bool HasStructuralDrift,
    IReadOnlyList<StackDrift> Drifts);

[JsonPolymorphic]
[DynamicallyAccessedMembers(DynamicallyAccessedMemberTypes.All)]
[JsonDerivedType(typeof(MissingContainer), nameof(MissingContainer))]
[JsonDerivedType(typeof(ExtraContainer), nameof(ExtraContainer))]
[JsonDerivedType(typeof(ContainerStopped), nameof(ContainerStopped))]
[JsonDerivedType(typeof(ContainerPaused), nameof(ContainerPaused))]
[JsonDerivedType(typeof(ContainerUnhealthy), nameof(ContainerUnhealthy))]
[JsonDerivedType(typeof(ImageMismatch), nameof(ImageMismatch))]
[JsonDerivedType(typeof(ConfigHashMismatch), nameof(ConfigHashMismatch))]
public abstract record StackDrift;

public sealed record MissingContainer(string ServiceName) : StackDrift;

public sealed record ExtraContainer(string ContainerId, string ServiceName) : StackDrift;

public sealed record ContainerStopped(string ContainerId, string ServiceName) : StackDrift;

public sealed record ContainerPaused(string ContainerId, string ServiceName) : StackDrift;

public sealed record ContainerUnhealthy(
    string ContainerId,
    string ServiceName,
    string? HealthStatus) : StackDrift;

public sealed record ImageMismatch(
    string ServiceName,
    string ExpectedImage,
    string ActualImage) : StackDrift;

public sealed record ConfigHashMismatch(
    string ServiceName,
    string? ExpectedHash,
    string? ActualHash) : StackDrift;

public sealed record StackReconciliationResult(
    Guid StackId,
    StackReconciliationStatus Status,
    StackDriftReport BeforeReport,
    StackDriftReport? AfterReport,
    IReadOnlyList<StackReconciliationAction> Actions);

public sealed record StackReconciliationAction(
    string ContainerId,
    string ServiceName,
    StackReconciliationActionType Action,
    bool Succeeded,
    string? ErrorMessage = null);
