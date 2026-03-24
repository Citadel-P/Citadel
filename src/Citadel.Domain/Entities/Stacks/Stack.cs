using System.Diagnostics.CodeAnalysis;
using System.Text.Json.Serialization;

namespace Domain.Entities.Stacks;

public sealed class Stack : IAuditedEntity, IReconcilableResource
{
    public Guid Id { get; private set; } = Guid.CreateVersion7();
    public Guid CurrentStackReleaseId { get; private set; }
    public string Name { get; private set; }
    public string? Description { get; private set; }
    public StackSource StackSource { get; private set; }
    public StackUpdateState StackUpdateState { get; private set; }

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
    public StackRelease? CurrentStackRelease { get; private set; } = null!;

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

    public void MarkProcessing(Guid controlTriggeredBy)
    {
        if (ControlState == ResourceControlState.Processing) return;
        
        if (CurrentStackRelease is null)
        {
            throw new InvalidOperationException("Cannot mark stack as processing without a current stack release.");
        }

        CurrentStackRelease.UpdateStackStatus(StackReleaseStatus.Pending);
        ControlTriggeredBy = controlTriggeredBy;
        ControlState = ResourceControlState.Processing;
        ControlStartedAt = DateTimeOffset.UtcNow.ToUnixTimeSeconds();
    }

    public void ReleaseProcessing(StackReleaseStatus status)
    {
        if (CurrentStackRelease is null)
        {
            throw new InvalidOperationException("Cannot mark stack as processing without a current stack release.");
        }

        CurrentStackRelease.UpdateStackStatus(status);
        ControlState = ResourceControlState.Idle;
        ControlStartedAt = null;
        ControlTriggeredBy = null;
    }
}

[JsonPolymorphic]
[DynamicallyAccessedMembers(DynamicallyAccessedMemberTypes.All)]
[JsonDerivedType(typeof(GitStackUpdateState), nameof(StackSource.Git))]
[JsonDerivedType(typeof(ManualStackUpdateState), nameof(StackSource.Manual))]
public abstract record StackUpdateState;

public sealed record ManualStackUpdateState(RecreateStackOnNewImageState RecreateStackOnNewImageState) : StackUpdateState;
public abstract record GitStackUpdateState(
    RecreateStackOnNewImageState RecreateStackOnNewImageState,
    RecreateStackOnNewCommitState RecreateStackOnNewCommitState
    ) : StackUpdateState;

public sealed record RecreateStackOnNewImageState(StackUpdateBehavior UpdateBehavior, IReadOnlyList<ImageUpdateState> AutoUpdateStates);
public sealed record RecreateStackOnNewCommitState(StackUpdateBehavior UpdateBehavior, string CurrentCommitSha, string? RemoteCommitSha, DateTime LastCheckedAt);


