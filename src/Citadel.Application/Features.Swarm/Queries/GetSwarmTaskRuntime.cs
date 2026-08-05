using Domain;
using Domain.Contracts.Interfaces;
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
public sealed record InspectSwarmTask(Guid PlatformId, string TaskId) : IQuery<Result<SwarmTaskResult>>
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
    IConnectorFactory<ISwarmConnector> connectorFactory)
    : IQueryHandler<InspectSwarmTask, Result<SwarmTaskResult>>
{
    public async ValueTask<Result<SwarmTaskResult>> Handle(
        InspectSwarmTask query,
        CancellationToken cancellationToken)
    {
        var context = await SwarmTaskRuntimeQuery.LoadAsync(
            unitOfWork,
            query.PlatformId,
            query.TaskId,
            cancellationToken);
        if (!context.IsSuccess(out var platform, out var error))
            return Result.Failure<SwarmTaskResult>(error!);

        var inspected = await connectorFactory.GetConnector(platform.ConnectorType).InspectTaskAsync(
            new InspectSwarmTaskCommand(platform.Address, query.TaskId),
            cancellationToken);
        if (!inspected.IsSuccess(out var task, out error))
            return Result.Failure<SwarmTaskResult>(error!);
        if (!string.Equals(task.Id, query.TaskId, StringComparison.OrdinalIgnoreCase))
        {
            return Result.Failure<SwarmTaskResult>(
                new InternalServerError("Docker returned a different task than the one requested."));
        }

        return Result.Success(task);
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
            container.DockerContainerId,
            query.Hours,
            cancellationToken);
        return Result.Success(new SwarmTaskStatsResult(container.DockerContainerId, stats.ToArray()));
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
        var context = await SwarmTaskRuntimeQuery.LoadRunningContainerAsync(
            unitOfWork,
            swarmConnectorFactory,
            query.PlatformId,
            query.TaskId,
            cancellationToken);
        return context.IsSuccess(out var container, out var error)
            ? Result.Success(container.DockerContainerId)
            : Result.Failure<string>(error!);
    }
}

internal static class SwarmTaskRuntimeQuery
{
    public static async Task<Result<Platform>> LoadAsync(
        IUnitOfWork unitOfWork,
        Guid platformId,
        string taskId,
        CancellationToken cancellationToken)
    {
        var platform = await unitOfWork.Platforms.GetByIdAsync(platformId, cancellationToken);
        if (platform is null)
            return Result.Failure<Platform>(new NotFoundError("Platform does not exist."));
        if (platform.PlatformDescriptor is not DockerSwarmPlatformDescriptor)
            return Result.Failure<Platform>(new BadRequestError("Task runtime data is only available for Docker Swarm platforms."));
        if (platform.Status != PlatformStatus.Online)
            return Result.Failure<Platform>(new ConflictError("Platform is offline."));
        if (await unitOfWork.Swarm.GetTaskAsync(platformId, taskId, cancellationToken) is null)
            return Result.Failure<Platform>(new NotFoundError("Swarm task does not exist."));

        return Result.Success(platform);
    }

    public static async Task<Result<Container>> LoadRunningContainerAsync(
        IUnitOfWork unitOfWork,
        IConnectorFactory<ISwarmConnector> connectorFactory,
        Guid platformId,
        string taskId,
        CancellationToken cancellationToken)
    {
        var loaded = await LoadAsync(unitOfWork, platformId, taskId, cancellationToken);
        if (!loaded.IsSuccess(out var platform, out var error))
            return Result.Failure<Container>(error!);

        var inspected = await connectorFactory.GetConnector(platform.ConnectorType).InspectTaskAsync(
            new InspectSwarmTaskCommand(platform.Address, taskId),
            cancellationToken);
        if (!inspected.IsSuccess(out var task, out error))
            return Result.Failure<Container>(error!);
        if (!string.Equals(task.Id, taskId, StringComparison.OrdinalIgnoreCase))
        {
            return Result.Failure<Container>(
                new InternalServerError("Docker returned a different task than the one requested."));
        }

        if (!string.Equals(task.State, "Running", StringComparison.OrdinalIgnoreCase)
            || string.IsNullOrWhiteSpace(task.ContainerId))
        {
            return Result.Failure<Container>(
                new ConflictError("Runtime access is only available while the task container is running."));
        }

        var descriptor = (DockerSwarmPlatformDescriptor)platform.PlatformDescriptor;
        if (!string.Equals(task.NodeId, descriptor.NodeID, StringComparison.OrdinalIgnoreCase))
        {
            return Result.Failure<Container>(
                new ConflictError("Docker exposes task runtime access only on the node running the task. This task is not running on the connected manager."));
        }

        var container = await unitOfWork.Containers.GetByIdAsync(task.ContainerId, cancellationToken);
        if (container is null || container.PlatformId != platformId)
        {
            return Result.Failure<Container>(
                new ConflictError("The running task container has not been synchronized yet."));
        }

        return Result.Success(container);
    }
}
