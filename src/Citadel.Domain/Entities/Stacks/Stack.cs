using System.Diagnostics.CodeAnalysis;
using System.Text.Json.Serialization;

namespace Domain.Entities.Stacks;

public sealed class Stack : IAuditedEntity, IReconcilableResource
{
    public Guid Id { get; private set; } = Guid.CreateVersion7();
    public Guid CurrentStackReleaseId { get; private set; }
    public string Name { get; private set; } = string.Empty;
    public string? Description { get; private set; }
    public StackSource StackSource { get; private set; }
    public StackUpdateState StackUpdateState { get; private set; } = new ManualStackUpdateState(new RecreateStackOnNewImageState([]));

    #region IAuditedEntity Members
    public DateTime CreatedAt { get; private set; } = DateTime.UtcNow;
    public Guid CreatedByActorId { get; private set; }
    #endregion
    
    #region IReconcilableResource Members
    public ResourceControlState ControlState { get; private set; } = ResourceControlState.Idle;
    public Guid? ControlTriggeredBy { get; private set; }
    public long? ControlStartedAt { get; private set; }
    public long RowVersion { get; private set; }
    #endregion
    
    public StackRelease? CurrentStackRelease { get; private set; }

    public static Stack Create(
        string name,
        Guid createdByActorId,
        StackSource StackSource,
        Guid platformId,
        StackSpec spec,
        string? description = null)
    {
        var stack = new Stack
        {
            Name = name,
            Description = description,
            StackSource = StackSource,
            CreatedByActorId = createdByActorId,
            StackUpdateState = CreateDefaultUpdateState(StackSource, spec),
        };

        stack.CurrentStackRelease = StackRelease.Create(
            stackId: stack.Id,
            platformId: platformId,
            spec: spec,
            createdByActorId: createdByActorId,
            version: "1");

        stack.CurrentStackReleaseId = stack.CurrentStackRelease.Id;

        return stack;
    }

    public static Stack FromPersistence(
        Guid id,
        Guid currentStackReleaseId,
        string name,
        string? description,
        StackSource stackSource,
        StackUpdateState stackUpdateState,
        DateTime createdAt,
        Guid createdByActorId,
        ResourceControlState controlState,
        Guid? controlTriggeredBy,
        long? controlStartedAt,
        long rowVersion,
        StackRelease? currentStackRelease = null)
    {
        return new Stack
        {
            Id = id,
            CurrentStackReleaseId = currentStackReleaseId,
            Name = name,
            Description = description,
            StackSource = stackSource,
            StackUpdateState = stackUpdateState,
            CreatedAt = createdAt,
            CreatedByActorId = createdByActorId,
            ControlState = controlState,
            ControlTriggeredBy = controlTriggeredBy,
            ControlStartedAt = controlStartedAt,
            RowVersion = rowVersion,
            CurrentStackRelease = currentStackRelease
        };
    }

    public bool MarkProcessing(Guid controlTriggeredBy)
    {
        if (ControlState == ResourceControlState.Processing || CurrentStackRelease is null) return false;

        CurrentStackRelease.UpdateStackStatus(StackReleaseStatus.Pending);
        ControlTriggeredBy = controlTriggeredBy;
        ControlState = ResourceControlState.Processing;
        ControlStartedAt = DateTimeOffset.UtcNow.ToUnixTimeSeconds();
        return true;
    }

    public bool ReleaseProcessing(StackReleaseStatus status)
    {
        if (CurrentStackRelease is null) return false;

        CurrentStackRelease.UpdateStackStatus(status);
        ControlState = ResourceControlState.Idle;
        ControlStartedAt = null;
        ControlTriggeredBy = null;
        return true;
    }

    public void UpdateDetails(
        string? name = null,
        string? description = null)
    {
        if (name != null) Name = name;
        if(description != null) Description = description;
    }

    public void SetCurrentStackRelease(StackRelease stackRelease)
    {
        CurrentStackRelease = stackRelease;
        CurrentStackReleaseId = stackRelease.Id;
    }

    private static StackUpdateState CreateDefaultUpdateState(StackSource stackSource, StackSpec spec)
    {
        var recreateOnNewImage = new RecreateStackOnNewImageState([]);

        return stackSource switch
        {
            StackSource.WebEditor => new ManualStackUpdateState(recreateOnNewImage),
            StackSource.Git => new GitStackUpdateState(
                RecreateStackOnNewImageState: recreateOnNewImage,
                RecreateStackOnNewCommitState: new RecreateStackOnNewCommitState(
                    CurrentCommitSha: spec is GitStack gitSpec ? gitSpec.CommitSha ?? string.Empty : string.Empty,
                    RemoteCommitSha: null,
                    LastCheckedAt: DateTime.MinValue)),
            _ => new ManualStackUpdateState(recreateOnNewImage)
        };
    }
}

[JsonPolymorphic]
[DynamicallyAccessedMembers(DynamicallyAccessedMemberTypes.All)]
[JsonDerivedType(typeof(GitStackUpdateState), nameof(StackSource.Git))]
[JsonDerivedType(typeof(ManualStackUpdateState), nameof(StackSource.WebEditor))]
public abstract record StackUpdateState;

public sealed record ManualStackUpdateState(RecreateStackOnNewImageState RecreateStackOnNewImageState) : StackUpdateState;
public sealed record GitStackUpdateState(
    RecreateStackOnNewImageState RecreateStackOnNewImageState,
    RecreateStackOnNewCommitState RecreateStackOnNewCommitState
    ) : StackUpdateState;

public sealed record RecreateStackOnNewImageState(IReadOnlyList<ImageUpdateState> AutoUpdateStates);
public sealed record RecreateStackOnNewCommitState(string CurrentCommitSha, string? RemoteCommitSha, DateTime LastCheckedAt);

public sealed record StackPatchModel(
    string? Name = null,
    Guid? PlatformId = null,
    string? Description = null,
    StackSource? StackSource = null,
    StackSpec? Spec = null);


