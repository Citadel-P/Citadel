using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Swarm;
using Domain.Entities.Platforms;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Swarm.Queries;

[RequirePermission(ResourceType.Platform, PermissionLevel.Read, SpecificPermission.Logs)]
public sealed record GetSwarmServiceLogs(
    Guid PlatformId,
    string ServiceId,
    int Tail = SwarmInventoryLimits.DefaultLogLines) : IQuery<Result<SwarmLogsResult>>
{
    internal sealed class Validator : AbstractValidator<GetSwarmServiceLogs>
    {
        public Validator()
        {
            RuleFor(query => query.PlatformId).NotEmpty();
            RuleFor(query => query.ServiceId).NotEmpty().MaximumLength(255);
            RuleFor(query => query.Tail).InclusiveBetween(1, SwarmInventoryLimits.MaximumLogLines);
        }
    }
}

[RequirePermission(ResourceType.Platform, PermissionLevel.Read, SpecificPermission.Logs)]
public sealed record GetSwarmTaskLogs(
    Guid PlatformId,
    string TaskId,
    int Tail = SwarmInventoryLimits.DefaultLogLines) : IQuery<Result<SwarmLogsResult>>
{
    internal sealed class Validator : AbstractValidator<GetSwarmTaskLogs>
    {
        public Validator()
        {
            RuleFor(query => query.PlatformId).NotEmpty();
            RuleFor(query => query.TaskId).NotEmpty().MaximumLength(255);
            RuleFor(query => query.Tail).InclusiveBetween(1, SwarmInventoryLimits.MaximumLogLines);
        }
    }
}

internal sealed class GetSwarmServiceLogsHandler(
    IUnitOfWork unitOfWork,
    IConnectorFactory<ISwarmConnector> connectorFactory)
    : IQueryHandler<GetSwarmServiceLogs, Result<SwarmLogsResult>>
{
    public async ValueTask<Result<SwarmLogsResult>> Handle(GetSwarmServiceLogs query, CancellationToken cancellationToken)
    {
        var context = await SwarmLogQuery.LoadPlatformAsync(unitOfWork, query.PlatformId, cancellationToken);
        if (!context.IsSuccess(out var platform, out var error))
            return Result.Failure<SwarmLogsResult>(error!);

        if (await unitOfWork.Swarm.GetServiceAsync(query.PlatformId, query.ServiceId, cancellationToken) is null)
            return Result.Failure<SwarmLogsResult>(new NotFoundError("Swarm service does not exist."));

        return await connectorFactory.GetConnector(platform.ConnectorType).GetServiceLogsAsync(
            new GetSwarmServiceLogsCommand(platform.Address, query.ServiceId, query.Tail),
            cancellationToken);
    }
}

internal sealed class GetSwarmTaskLogsHandler(
    IUnitOfWork unitOfWork,
    IConnectorFactory<ISwarmConnector> connectorFactory)
    : IQueryHandler<GetSwarmTaskLogs, Result<SwarmLogsResult>>
{
    public async ValueTask<Result<SwarmLogsResult>> Handle(GetSwarmTaskLogs query, CancellationToken cancellationToken)
    {
        var context = await SwarmLogQuery.LoadPlatformAsync(unitOfWork, query.PlatformId, cancellationToken);
        if (!context.IsSuccess(out var platform, out var error))
            return Result.Failure<SwarmLogsResult>(error!);

        if (await unitOfWork.Swarm.GetTaskAsync(query.PlatformId, query.TaskId, cancellationToken) is null)
            return Result.Failure<SwarmLogsResult>(new NotFoundError("Swarm task does not exist."));

        return await connectorFactory.GetConnector(platform.ConnectorType).GetTaskLogsAsync(
            new GetSwarmTaskLogsCommand(platform.Address, query.TaskId, query.Tail),
            cancellationToken);
    }
}

internal static class SwarmLogQuery
{
    public static async Task<Result<Platform>> LoadPlatformAsync(
        IUnitOfWork unitOfWork,
        Guid platformId,
        CancellationToken cancellationToken)
    {
        var platform = await unitOfWork.Platforms.GetByIdAsync(platformId, cancellationToken);
        if (platform is null)
            return Result.Failure<Platform>(new NotFoundError("Platform does not exist."));
        if (platform.PlatformDescriptor is not DockerSwarmPlatformDescriptor)
            return Result.Failure<Platform>(new BadRequestError("Swarm logs are only available for Docker Swarm platforms."));
        if (platform.Status != PlatformStatus.Online)
            return Result.Failure<Platform>(new ConflictError("Platform is offline."));
        return Result.Success(platform);
    }
}
