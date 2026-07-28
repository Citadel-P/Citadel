using Application.Features.Builds.Models;
using Application.Features.Deployments.Notifications;
using Application.Services;
using Application.Services.Builds;
using Application.Services.SignalR;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Images;
using Domain.Contracts.Resources.Platforms;
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

[RequirePermission(ResourceType.BuildAgentPool, PermissionLevel.Write, ResourceIdProperty = nameof(UpdateBuildAgentPool.PoolId))]
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

[RequirePermission(ResourceType.BuildAgentPool, PermissionLevel.Write, ResourceIdProperty = nameof(RenameBuildAgentPool.PoolId))]
public sealed record RenameBuildAgentPool(Guid PoolId, string Name) : ICommand<Result<BuildAgentPoolResult>>;

[RequirePermission(ResourceType.BuildAgentPool, PermissionLevel.Write, ResourceIdProperty = nameof(PatchBuildAgentPoolMetadata.PoolId))]
public sealed record PatchBuildAgentPoolMetadata(Guid PoolId, string? Description) : ICommand<Result<BuildAgentPoolResult>>;

[RequirePermission(ResourceType.BuildAgentPool, PermissionLevel.Write, ResourceIdProperty = nameof(TestBuildAgentPool.PoolId))]
public sealed record TestBuildAgentPool(Guid PoolId) : ICommand<Result<BuildAgentPoolResult>>;

[RequirePermission(ResourceType.BuildAgentPool, PermissionLevel.Write, ResourceIdProperty = nameof(ArchiveBuildAgentPool.PoolId))]
public sealed record ArchiveBuildAgentPool(Guid PoolId) : ICommand<Result>;

[RequirePermission(ResourceType.BuildAgentPool, PermissionLevel.Write, ResourceIdProperty = nameof(CreateBuildAgentPoolEdgeEnrollment.PoolId))]
public sealed record CreateBuildAgentPoolEdgeEnrollment(Guid PoolId, string CoreUrl) : ICommand<Result<EdgeAgentEnrollmentResult>>
{
    internal sealed class Validator : AbstractValidator<CreateBuildAgentPoolEdgeEnrollment>
    {
        public Validator()
        {
            RuleFor(x => x.PoolId).NotEmpty();
            RuleFor(x => x.CoreUrl)
                .NotEmpty()
                .Must(x => Uri.TryCreate(x, UriKind.Absolute, out var uri)
                           && (uri.Scheme == Uri.UriSchemeHttp || uri.Scheme == Uri.UriSchemeHttps))
                .WithMessage("CoreUrl must be an absolute HTTP or HTTPS URL.");
        }
    }
}

[RequirePermission(ResourceType.BuildAgentPool, PermissionLevel.Read, ResourceIdProperty = nameof(GetBuildAgentPoolEdgeStatus.PoolId))]
public sealed record GetBuildAgentPoolEdgeStatus(Guid PoolId) : IQuery<Result<EdgeAgentStatusResult>>;

[RequirePermission(ResourceType.BuildAgentPool, PermissionLevel.Execute, ResourceIdProperty = nameof(RevokeBuildAgentPoolEdgeAgent.PoolId))]
public sealed record RevokeBuildAgentPoolEdgeAgent(Guid PoolId) : ICommand<Result>;

internal sealed class CreateBuildAgentPoolHandler(
    IUnitOfWork unitOfWork,
    IUserContextAccessor userContextAccessor,
    IBuildAgentPoolStreamManager buildAgentPoolStreamManager,
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
        await buildAgentPoolStreamManager.SendBuildAgentPoolInfo(pool, "create");
        await notificationQueue.EnqueueAsync(new ActivityNotificationWorkItem(activityHub, await activity.AssignActor(unitOfWork, cancellationToken)), cancellationToken);
        return Result.Success(new BuildAgentPoolResult(pool));
    }
}

internal sealed class UpdateBuildAgentPoolHandler(
    IUnitOfWork unitOfWork,
    IUserContextAccessor userContextAccessor,
    IBuildAgentPoolStreamManager buildAgentPoolStreamManager,
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
        await buildAgentPoolStreamManager.SendBuildAgentPoolInfo(pool);
        await notificationQueue.EnqueueAsync(new ActivityNotificationWorkItem(activityHub, await activity.AssignActor(unitOfWork, cancellationToken)), cancellationToken);
        return Result.Success(new BuildAgentPoolResult(pool));
    }
}

internal sealed class RenameBuildAgentPoolHandler(
    IUnitOfWork unitOfWork,
    IUserContextAccessor userContextAccessor,
    IBuildAgentPoolStreamManager buildAgentPoolStreamManager,
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
        await buildAgentPoolStreamManager.SendBuildAgentPoolInfo(pool);
        await notificationQueue.EnqueueAsync(new ActivityNotificationWorkItem(activityHub, await activity.AssignActor(unitOfWork, cancellationToken)), cancellationToken);
        return Result.Success(new BuildAgentPoolResult(pool));
    }
}

internal sealed class ArchiveBuildAgentPoolHandler(
    IUnitOfWork unitOfWork,
    IUserContextAccessor userContextAccessor,
    IBuildAgentPoolStreamManager buildAgentPoolStreamManager,
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
        await buildAgentPoolStreamManager.SendBuildAgentPoolInfo(pool, "delete");
        await notificationQueue.EnqueueAsync(new ActivityNotificationWorkItem(activityHub, await activity.AssignActor(unitOfWork, cancellationToken)), cancellationToken);
        return Result.Success();
    }
}

internal sealed class CreateBuildAgentPoolEdgeEnrollmentHandler(
    IEdgeAgentManagementService edgeAgentManagementService,
    IUserContextAccessor userContextAccessor)
    : ICommandHandler<CreateBuildAgentPoolEdgeEnrollment, Result<EdgeAgentEnrollmentResult>>
{
    private static readonly TimeSpan EnrollmentTtl = TimeSpan.FromHours(24);

    public ValueTask<Result<EdgeAgentEnrollmentResult>> Handle(CreateBuildAgentPoolEdgeEnrollment command, CancellationToken cancellationToken)
        => new(edgeAgentManagementService.CreateBuildAgentPoolEnrollmentAsync(
            command.PoolId,
            command.CoreUrl.TrimEnd('/'),
            userContextAccessor.Current.ActorId,
            EnrollmentTtl,
            cancellationToken));
}

internal sealed class GetBuildAgentPoolEdgeStatusHandler(IEdgeAgentManagementService edgeAgentManagementService)
    : IQueryHandler<GetBuildAgentPoolEdgeStatus, Result<EdgeAgentStatusResult>>
{
    public ValueTask<Result<EdgeAgentStatusResult>> Handle(GetBuildAgentPoolEdgeStatus query, CancellationToken cancellationToken)
        => new(edgeAgentManagementService.GetBuildAgentPoolStatusAsync(query.PoolId, DateTime.UtcNow, cancellationToken));
}

internal sealed class RevokeBuildAgentPoolEdgeAgentHandler(IEdgeAgentManagementService edgeAgentManagementService)
    : ICommandHandler<RevokeBuildAgentPoolEdgeAgent, Result>
{
    public ValueTask<Result> Handle(RevokeBuildAgentPoolEdgeAgent command, CancellationToken cancellationToken)
        => new(edgeAgentManagementService.RevokeBuildAgentPoolAsync(command.PoolId, DateTime.UtcNow, cancellationToken));
}

internal sealed class PatchBuildAgentPoolMetadataHandler(
    IUnitOfWork unitOfWork,
    IUserContextAccessor userContextAccessor,
    IBuildAgentPoolStreamManager buildAgentPoolStreamManager,
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
        await buildAgentPoolStreamManager.SendBuildAgentPoolInfo(pool);
        await notificationQueue.EnqueueAsync(new ActivityNotificationWorkItem(activityHub, await activity.AssignActor(unitOfWork, cancellationToken)), cancellationToken);
        return Result.Success(new BuildAgentPoolResult(pool));
    }
}

internal sealed class TestBuildAgentPoolHandler(
    IUnitOfWork unitOfWork,
    IUserContextAccessor userContextAccessor,
    IBuildAgentPoolValidationService validationService,
    IBuildAgentPoolStreamManager buildAgentPoolStreamManager,
    IActivityStreamManager activityHub,
    INotificationQueue notificationQueue)
    : ICommandHandler<TestBuildAgentPool, Result<BuildAgentPoolResult>>
{
    public async ValueTask<Result<BuildAgentPoolResult>> Handle(TestBuildAgentPool command, CancellationToken cancellationToken)
    {
        var pool = await unitOfWork.BuildAgentPools.GetAsync(command.PoolId, cancellationToken);
        if (pool is null)
            return Result.Failure<BuildAgentPoolResult>(new NotFoundError("Build pool not found."));

        if (pool.ProviderSpec is not SelfManagedVmBuildAgentPoolProviderSpec)
            return Result.Failure<BuildAgentPoolResult>(new ConflictError("Only self-managed Citadel Agent build pools can be tested."));

        if (pool.ControlState == ResourceControlState.Processing)
            return Result.Failure<BuildAgentPoolResult>(new ConflictError("Build pool test is already running."));

        var now = DateTimeOffset.UtcNow;
        var rowVersion = pool.RowVersion;
        pool.MarkProcessing(userContextAccessor.Current.ActorId, now);
        var marked = await unitOfWork.BuildAgentPools.UpdateProcessingAsync(
            pool.Id,
            pool.ControlState,
            pool.ControlStartedAt,
            rowVersion,
            checkRowVersion: true,
            pool.ControlTriggeredBy,
            cancellationToken);
        if (marked == 0)
            return Result.Failure<BuildAgentPoolResult>(new ConflictError("Build pool test is already running."));

        await unitOfWork.CommitAsync(cancellationToken);
        await buildAgentPoolStreamManager.SendBuildAgentPoolInfo(pool);

        var validation = await validationService.ValidateAsync(pool, cancellationToken);
        now = DateTimeOffset.UtcNow;
        pool.ApplyValidation(validation.Status, validation.Message, now);
        pool.MarkIdle(now);

        var activity = BuildAgentPoolActivity.Create(
            pool,
            userContextAccessor.Current.ActorId,
            ActivityEventType.BuildAgentPoolTested,
            new BuildAgentPoolTested(pool.ToSnapshot(), validation.Status, validation.Message));

        await unitOfWork.ActivityEventRepository.AddAsync(activity, cancellationToken);
        await unitOfWork.BuildAgentPools.UpdateAsync(pool, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);
        await buildAgentPoolStreamManager.SendBuildAgentPoolInfo(pool);
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
