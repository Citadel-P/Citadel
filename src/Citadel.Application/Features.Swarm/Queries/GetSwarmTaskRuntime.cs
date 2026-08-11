using Application.Services;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using Domain.Contracts.Resources.Swarm;
using Domain.Entities;
using Domain.Entities.Platforms;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Swarm.Queries;

[RequirePermission(ResourceType.Platform, PermissionLevel.Read, SpecificPermission.Inspect)]
public sealed record InspectSwarmTask(Guid PlatformId, string TaskId) : IQuery<Result<ContainerInspectionInfo>>
{
    internal sealed class Validator : AbstractValidator<InspectSwarmTask>
    {
        public Validator()
        {
            RuleFor(query => query.PlatformId).NotEmpty();
            RuleFor(query => query.TaskId).NotEmpty().MaximumLength(255);
        }
    }
}

[RequirePermission(ResourceType.Platform, PermissionLevel.Read)]
public sealed record GetSwarmTaskStats(Guid PlatformId, string TaskId, int Hours = 24)
    : IQuery<Result<SwarmTaskStatsResult>>
{
    internal sealed class Validator : AbstractValidator<GetSwarmTaskStats>
    {
        public Validator()
        {
            RuleFor(query => query.PlatformId).NotEmpty();
            RuleFor(query => query.TaskId).NotEmpty().MaximumLength(255);
            RuleFor(query => query.Hours)
                .Must(hours => hours is 24 or 48 or 72)
                .WithMessage("Hours must be one of: 24, 48, 72.");
        }
    }
}

[RequirePermission(ResourceType.Platform, PermissionLevel.Read, SpecificPermission.Terminal)]
public sealed record GetSwarmTaskTerminalTarget(Guid PlatformId, string TaskId) : IQuery<Result<string>>
{
    internal sealed class Validator : AbstractValidator<GetSwarmTaskTerminalTarget>
    {
        public Validator()
        {
            RuleFor(query => query.PlatformId).NotEmpty();
            RuleFor(query => query.TaskId).NotEmpty().MaximumLength(255);
        }
    }
}

internal sealed class InspectSwarmTaskHandler(
    IUnitOfWork unitOfWork,
    IConnectorFactory<ISwarmConnector> connectorFactory,
    ISwarmNodeRuntimeConnector swarmNodeRuntimeConnector)
    : IQueryHandler<InspectSwarmTask, Result<ContainerInspectionInfo>>
{
    public async ValueTask<Result<ContainerInspectionInfo>> Handle(
        InspectSwarmTask query,
        CancellationToken cancellationToken)
    {
        var context = await SwarmTaskRuntimeQuery.LoadRunningTargetAsync(
            unitOfWork,
            connectorFactory,
            query.PlatformId,
            query.TaskId,
            cancellationToken);
        if (!context.IsSuccess(out var target, out var error))
            return Result.Failure<ContainerInspectionInfo>(error!);

        var inspected = await swarmNodeRuntimeConnector.InspectContainerAsync(
            target.Platform,
            target.DockerNodeId,
            target.DockerContainerId,
            cancellationToken);
        return inspected.IsSuccess(out var inspection, out error)
            ? Result.Success(ContainerInspectionRedactor.RedactEnvironment(inspection))
            : Result.Failure<ContainerInspectionInfo>(error!);
    }
}

internal sealed class GetSwarmTaskStatsHandler(
    IUnitOfWork unitOfWork,
    IConnectorFactory<ISwarmConnector> swarmConnectorFactory)
    : IQueryHandler<GetSwarmTaskStats, Result<SwarmTaskStatsResult>>
{
    public async ValueTask<Result<SwarmTaskStatsResult>> Handle(
        GetSwarmTaskStats query,
        CancellationToken cancellationToken)
    {
        var context = await SwarmTaskRuntimeQuery.LoadRunningContainerAsync(
            unitOfWork,
            swarmConnectorFactory,
            query.PlatformId,
            query.TaskId,
            cancellationToken);
        if (!context.IsSuccess(out var container, out var error))
            return Result.Failure<SwarmTaskStatsResult>(error!);

        var stats = await unitOfWork.ContainerStats.GetStatsAggregatedAsync(
            container.Id,
            query.Hours,
            cancellationToken);
        return Result.Success(new SwarmTaskStatsResult(container.Id, container.DockerContainerId, stats.ToArray()));
    }
}

internal sealed class GetSwarmTaskTerminalTargetHandler(
    IUnitOfWork unitOfWork,
    IConnectorFactory<ISwarmConnector> swarmConnectorFactory)
    : IQueryHandler<GetSwarmTaskTerminalTarget, Result<string>>
{
    public async ValueTask<Result<string>> Handle(
        GetSwarmTaskTerminalTarget query,
        CancellationToken cancellationToken)
    {
        var context = await SwarmTaskRuntimeQuery.LoadRunningTargetAsync(
            unitOfWork,
            swarmConnectorFactory,
            query.PlatformId,
            query.TaskId,
            cancellationToken);
        return context.IsSuccess(out var target, out var error)
            ? Result.Success(target.DockerContainerId)
            : Result.Failure<string>(error!);
    }
}

internal static class SwarmTaskRuntimeQuery
{
    private static async Task<Result<SwarmTaskRuntimeContext>> LoadAsync(
        IUnitOfWork unitOfWork,
        Guid platformId,
        string taskId,
        CancellationToken cancellationToken)
    {
        var platform = await unitOfWork.Platforms.GetByIdAsync(platformId, cancellationToken);
        if (platform is null)
            return Result.Failure<SwarmTaskRuntimeContext>(new NotFoundError("Platform does not exist."));
        if (platform.PlatformDescriptor is not DockerSwarmPlatformDescriptor)
            return Result.Failure<SwarmTaskRuntimeContext>(new BadRequestError("Task runtime data is only available for Docker Swarm platforms."));
        if (platform.Status != PlatformStatus.Online)
            return Result.Failure<SwarmTaskRuntimeContext>(new ConflictError("Platform is offline."));

        var task = await unitOfWork.Swarm.GetTaskAsync(platformId, taskId, cancellationToken);
        if (task is null)
            return Result.Failure<SwarmTaskRuntimeContext>(new NotFoundError("Swarm task does not exist."));

        return Result.Success(new SwarmTaskRuntimeContext(platform, task));
    }

    public static async Task<Result<Container>> LoadRunningContainerAsync(
        IUnitOfWork unitOfWork,
        IConnectorFactory<ISwarmConnector> connectorFactory,
        Guid platformId,
        string taskId,
        CancellationToken cancellationToken)
    {
        var loaded = await LoadRunningTargetAsync(
            unitOfWork,
            connectorFactory,
            platformId,
            taskId,
            cancellationToken);
        if (!loaded.IsSuccess(out var target, out var error))
            return Result.Failure<Container>(error!);

        var container = await unitOfWork.Containers.GetByRuntimeIdentityAsync(
            platformId,
            target.DockerNodeId,
            target.DockerContainerId,
            cancellationToken);
        if (container is null)
        {
            return Result.Failure<Container>(
                new ConflictError("The running task container has not been synchronized yet."));
        }

        return Result.Success(container);
    }

    public static async Task<Result<SwarmTaskRuntimeTarget>> LoadRunningTargetAsync(
        IUnitOfWork unitOfWork,
        IConnectorFactory<ISwarmConnector> connectorFactory,
        Guid platformId,
        string taskId,
        CancellationToken cancellationToken)
    {
        var loaded = await LoadAsync(unitOfWork, platformId, taskId, cancellationToken);
        if (!loaded.IsSuccess(out var context, out var error))
            return Result.Failure<SwarmTaskRuntimeTarget>(error!);

        var inspected = await connectorFactory.GetConnector(context.Platform.ConnectorType).InspectTaskAsync(
            new InspectSwarmTaskCommand(context.Platform.Address, taskId),
            cancellationToken);
        if (!inspected.IsSuccess(out var task, out error))
            return Result.Failure<SwarmTaskRuntimeTarget>(error!);
        if (!string.Equals(task.Id, taskId, StringComparison.OrdinalIgnoreCase))
        {
            return Result.Failure<SwarmTaskRuntimeTarget>(
                new InternalServerError("Docker returned a different task than the one requested."));
        }

        if (!string.Equals(task.State, "Running", StringComparison.OrdinalIgnoreCase)
            || string.IsNullOrWhiteSpace(task.ContainerId))
        {
            return Result.Failure<SwarmTaskRuntimeTarget>(
                new ConflictError("TaskNoLongerRunning: runtime access is only available while this exact task container is running."));
        }

        if (!string.Equals(context.Task.DockerNodeId, task.NodeId, StringComparison.Ordinal)
            || !string.Equals(context.Task.State, "Running", StringComparison.OrdinalIgnoreCase))
        {
            return Result.Failure<SwarmTaskRuntimeTarget>(
                new ConflictError("TaskNoLongerRunning: the persisted task observation no longer matches Docker's current runtime target."));
        }

        return Result.Success(new SwarmTaskRuntimeTarget(context.Platform, task.NodeId, task.ContainerId));
    }
}

internal sealed record SwarmTaskRuntimeContext(
    Platform Platform,
    SwarmTaskProjection Task);

internal sealed record SwarmTaskRuntimeTarget(
    Platform Platform,
    string DockerNodeId,
    string DockerContainerId);
