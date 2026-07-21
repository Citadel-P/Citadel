using Application.Features.Builds.Models;
using Application.Features.Deployments.Notifications;
using Application.Services;
using Application.Services.SignalR;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Activities;
using Domain.Entities.Builds;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Builds.Commands;

[RequirePermission(ResourceType.BuildAgentPool, PermissionLevel.Write)]
public sealed record CreateBuildAgentPool(BuildAgentPoolInputModel Pool) : ICommand<Result<BuildAgentPoolResult>>
{
    internal sealed class Validator : AbstractValidator<CreateBuildAgentPool>
    {
        public Validator()
        {
            RuleFor(x => x.Pool.Name).NotEmpty().MaximumLength(128);
            RuleFor(x => x.Pool.Description).MaximumLength(600).When(x => x.Pool.Description is not null);
            RuleFor(x => x.Pool.ProviderSpec).NotNull();
        }
    }
}

[RequirePermission(ResourceType.BuildAgentPool, PermissionLevel.Write)]
public sealed record UpdateBuildAgentPool(
    Guid PoolId,
    UpdateBuildAgentPoolInputModel Pool,
    bool UpdateDescription) : ICommand<Result<BuildAgentPoolResult>>
{
    internal sealed class Validator : AbstractValidator<UpdateBuildAgentPool>
    {
        public Validator()
        {
            RuleFor(x => x.PoolId).NotEmpty();
            RuleFor(x => x.Pool.Description).MaximumLength(600).When(x => x.Pool.Description is not null);
        }
    }
}

[RequirePermission(ResourceType.BuildAgentPool, PermissionLevel.Write)]
public sealed record RenameBuildAgentPool(Guid PoolId, string Name) : ICommand<Result<BuildAgentPoolResult>>;

[RequirePermission(ResourceType.BuildAgentPool, PermissionLevel.Write)]
public sealed record PatchBuildAgentPoolMetadata(Guid PoolId, string? Description) : ICommand<Result<BuildAgentPoolResult>>;

[RequirePermission(ResourceType.BuildAgentPool, PermissionLevel.Write)]
public sealed record ArchiveBuildAgentPool(Guid PoolId) : ICommand<Result>;

internal sealed class CreateBuildAgentPoolHandler(
    IUnitOfWork unitOfWork,
    IUserContextAccessor userContextAccessor,
    IActivityStreamManager activityHub,
    INotificationQueue notificationQueue)
    : ICommandHandler<CreateBuildAgentPool, Result<BuildAgentPoolResult>>
{
    public async ValueTask<Result<BuildAgentPoolResult>> Handle(CreateBuildAgentPool command, CancellationToken cancellationToken)
    {
        var input = command.Pool;
        var actorId = userContextAccessor.Current.ActorId;
        var normalizedName = BuildProject.ToNormalizedName(input.Name);
        if (await unitOfWork.BuildAgentPools.ExistsByNormalizedNameAsync(normalizedName, cancellationToken))
            return Result.Failure<BuildAgentPoolResult>(new ConflictError("Build pool name already exists."));

        var pool = new BuildAgentPool(
            input.Name,
            input.Description,
            input.Enabled,
            input.ProviderSpec,
            input.MaxActiveBuilders ?? BuildAgentPool.DefaultMaxActiveBuilders,
            input.QueueTimeoutSeconds ?? BuildAgentPool.DefaultQueueTimeoutSeconds,
            input.ProvisioningTimeoutSeconds ?? BuildAgentPool.DefaultProvisioningTimeoutSeconds,
            input.RegistrationTimeoutSeconds ?? BuildAgentPool.DefaultRegistrationTimeoutSeconds,
            input.HeartbeatTimeoutSeconds ?? BuildAgentPool.DefaultHeartbeatTimeoutSeconds,
            input.CleanupTimeoutSeconds ?? BuildAgentPool.DefaultCleanupTimeoutSeconds,
            input.MaximumInstanceLifetimeSeconds ?? BuildAgentPool.DefaultMaximumInstanceLifetimeSeconds,
            input.FailureRetentionMinutes ?? BuildAgentPool.DefaultFailureRetentionMinutes,
            actorId);

        try
        {
            pool.Validate();
        }
        catch (ArgumentException ex)
        {
            return Result.Failure<BuildAgentPoolResult>(new BadRequestError(ex.Message));
        }

        var activity = BuildAgentPoolActivity.Create(pool, actorId, ActivityEventType.BuildAgentPoolCreated, new BuildAgentPoolCreated(pool.ToSnapshot()));

        await unitOfWork.ActivityEventRepository.AddAsync(activity, cancellationToken);
        var rows = await unitOfWork.BuildAgentPools.AddAsync(pool, cancellationToken, input.TagIds, actorId);
        if (rows == 0)
            return Result.Failure<BuildAgentPoolResult>(new BadRequestError("One or more tags do not exist."));

        await unitOfWork.CommitAsync(cancellationToken);
        await notificationQueue.EnqueueAsync(new ActivityNotificationWorkItem(activityHub, await activity.AssignActor(unitOfWork, cancellationToken)), cancellationToken);
        return Result.Success(new BuildAgentPoolResult(pool));
    }
}

internal sealed class UpdateBuildAgentPoolHandler(
    IUnitOfWork unitOfWork,
    IUserContextAccessor userContextAccessor,
    IActivityStreamManager activityHub,
    INotificationQueue notificationQueue)
    : ICommandHandler<UpdateBuildAgentPool, Result<BuildAgentPoolResult>>
{
    public async ValueTask<Result<BuildAgentPoolResult>> Handle(UpdateBuildAgentPool command, CancellationToken cancellationToken)
    {
        var pool = await unitOfWork.BuildAgentPools.GetAsync(command.PoolId, cancellationToken);
        if (pool is null)
            return Result.Failure<BuildAgentPoolResult>(new NotFoundError("Build pool not found."));

        var oldSnapshot = pool.ToSnapshot();
        var input = command.Pool;
        try
        {
            pool.Update(
                command.UpdateDescription ? input.Description : pool.Description,
                input.Enabled,
                input.ProviderSpec,
                input.MaxActiveBuilders,
                input.QueueTimeoutSeconds,
                input.ProvisioningTimeoutSeconds,
                input.RegistrationTimeoutSeconds,
                input.HeartbeatTimeoutSeconds,
                input.CleanupTimeoutSeconds,
                input.MaximumInstanceLifetimeSeconds,
                input.FailureRetentionMinutes);
            pool.Validate();
        }
        catch (ArgumentException ex)
        {
            return Result.Failure<BuildAgentPoolResult>(new BadRequestError(ex.Message));
        }

        var activity = BuildAgentPoolActivity.Create(
            pool,
            userContextAccessor.Current.ActorId,
            ActivityEventType.BuildAgentPoolUpdated,
            new BuildAgentPoolUpdated(oldSnapshot, pool.ToSnapshot()));

        await unitOfWork.ActivityEventRepository.AddAsync(activity, cancellationToken);
        await unitOfWork.BuildAgentPools.UpdateAsync(pool, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);
        await notificationQueue.EnqueueAsync(new ActivityNotificationWorkItem(activityHub, await activity.AssignActor(unitOfWork, cancellationToken)), cancellationToken);
        return Result.Success(new BuildAgentPoolResult(pool));
    }
}

internal sealed class RenameBuildAgentPoolHandler(
    IUnitOfWork unitOfWork,
    IUserContextAccessor userContextAccessor,
    IActivityStreamManager activityHub,
    INotificationQueue notificationQueue)
    : ICommandHandler<RenameBuildAgentPool, Result<BuildAgentPoolResult>>
{
    public async ValueTask<Result<BuildAgentPoolResult>> Handle(RenameBuildAgentPool command, CancellationToken cancellationToken)
    {
        var pool = await unitOfWork.BuildAgentPools.GetAsync(command.PoolId, cancellationToken);
        if (pool is null)
            return Result.Failure<BuildAgentPoolResult>(new NotFoundError("Build pool not found."));

        var normalizedName = BuildProject.ToNormalizedName(command.Name);
        if (await unitOfWork.BuildAgentPools.ExistsByNormalizedNameExceptAsync(normalizedName, pool.Id, cancellationToken))
            return Result.Failure<BuildAgentPoolResult>(new ConflictError("Build pool name already exists."));

        var oldName = pool.Name;
        try
        {
            pool.Rename(command.Name);
            pool.Validate();
        }
        catch (ArgumentException ex)
        {
            return Result.Failure<BuildAgentPoolResult>(new BadRequestError(ex.Message));
        }

        var activity = BuildAgentPoolActivity.Create(
            pool,
            userContextAccessor.Current.ActorId,
            ActivityEventType.BuildAgentPoolRenamed,
            new BuildAgentPoolRenamed(oldName, pool.Name));

        await unitOfWork.ActivityEventRepository.AddAsync(activity, cancellationToken);
        await unitOfWork.BuildAgentPools.UpdateAsync(pool, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);
        await notificationQueue.EnqueueAsync(new ActivityNotificationWorkItem(activityHub, await activity.AssignActor(unitOfWork, cancellationToken)), cancellationToken);
        return Result.Success(new BuildAgentPoolResult(pool));
    }
}

internal sealed class ArchiveBuildAgentPoolHandler(
    IUnitOfWork unitOfWork,
    IUserContextAccessor userContextAccessor,
    IActivityStreamManager activityHub,
    INotificationQueue notificationQueue)
    : ICommandHandler<ArchiveBuildAgentPool, Result>
{
    public async ValueTask<Result> Handle(ArchiveBuildAgentPool command, CancellationToken cancellationToken)
    {
        var pool = await unitOfWork.BuildAgentPools.GetAsync(command.PoolId, cancellationToken);
        if (pool is null)
            return Result.Failure(new NotFoundError("Build pool not found."));

        var snapshot = pool.ToSnapshot();
        pool.Archive(DateTimeOffset.UtcNow);
        var activity = BuildAgentPoolActivity.Create(
            pool,
            userContextAccessor.Current.ActorId,
            ActivityEventType.BuildAgentPoolDeleted,
            new BuildAgentPoolDeleted(snapshot));

        await unitOfWork.ActivityEventRepository.AddAsync(activity, cancellationToken);
        await unitOfWork.BuildAgentPools.UpdateAsync(pool, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);
        await notificationQueue.EnqueueAsync(new ActivityNotificationWorkItem(activityHub, await activity.AssignActor(unitOfWork, cancellationToken)), cancellationToken);
        return Result.Success();
    }
}

internal sealed class PatchBuildAgentPoolMetadataHandler(
    IUnitOfWork unitOfWork,
    IUserContextAccessor userContextAccessor,
    IActivityStreamManager activityHub,
    INotificationQueue notificationQueue)
    : ICommandHandler<PatchBuildAgentPoolMetadata, Result<BuildAgentPoolResult>>
{
    public async ValueTask<Result<BuildAgentPoolResult>> Handle(PatchBuildAgentPoolMetadata command, CancellationToken cancellationToken)
    {
        var pool = await unitOfWork.BuildAgentPools.GetAsync(command.PoolId, cancellationToken);
        if (pool is null)
            return Result.Failure<BuildAgentPoolResult>(new NotFoundError("Build pool not found."));

        var oldSnapshot = pool.ToSnapshot();
        try
        {
            pool.Update(
                command.Description,
                null,
                null,
                null,
                null,
                null,
                null,
                null,
                null,
                null,
                null);
            pool.Validate();
        }
        catch (ArgumentException ex)
        {
            return Result.Failure<BuildAgentPoolResult>(new BadRequestError(ex.Message));
        }

        var activity = BuildAgentPoolActivity.Create(
            pool,
            userContextAccessor.Current.ActorId,
            ActivityEventType.BuildAgentPoolUpdated,
            new BuildAgentPoolUpdated(oldSnapshot, pool.ToSnapshot()));

        await unitOfWork.ActivityEventRepository.AddAsync(activity, cancellationToken);
        await unitOfWork.BuildAgentPools.UpdateAsync(pool, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);
        await notificationQueue.EnqueueAsync(new ActivityNotificationWorkItem(activityHub, await activity.AssignActor(unitOfWork, cancellationToken)), cancellationToken);
        return Result.Success(new BuildAgentPoolResult(pool));
    }
}

internal static class BuildAgentPoolActivity
{
    internal static ActivityEvent Create(
        BuildAgentPool pool,
        Guid actorId,
        ActivityEventType eventType,
        ActivityEventInfo info)
        => new(
            platformId: null,
            resourceId: pool.Id,
            actorId: actorId,
            resourceName: pool.Name,
            eventType: eventType,
            status: ActivityStatus.Success,
            info: info);
}
