using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Swarm;
using Domain.Entities.Platforms;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.SwarmServices.Queries;

[RequirePermission(ResourceType.SwarmService, PermissionLevel.Read, SpecificPermission.Inspect, ResourceIdProperty = nameof(Id))]
public sealed record InspectManagedSwarmService(Guid Id) : IQuery<Result<SwarmServiceResult>>;

[RequirePermission(ResourceType.SwarmService, PermissionLevel.Read, SpecificPermission.Logs, ResourceIdProperty = nameof(Id))]
public sealed record GetManagedSwarmServiceLogs(Guid Id, int Tail = SwarmInventoryLimits.DefaultLogLines)
    : IQuery<Result<SwarmLogsResult>>;

internal sealed class InspectManagedSwarmServiceHandler(
    IUnitOfWork unitOfWork,
    IConnectorFactory<ISwarmConnector> connectorFactory,
    IUserContextAccessor userContext)
    : IQueryHandler<InspectManagedSwarmService, Result<SwarmServiceResult>>
{
    public async ValueTask<Result<SwarmServiceResult>> Handle(
        InspectManagedSwarmService query,
        CancellationToken cancellationToken)
    {
        var context = await ManagedSwarmServiceRuntime.LoadAsync(
            query.Id, unitOfWork, userContext, cancellationToken);
        if (!context.IsSuccess(out var value, out var error))
            return Result.Failure<SwarmServiceResult>(error!);

        var inspected = await connectorFactory.GetConnector(value.Platform.ConnectorType).InspectServiceAsync(
            new InspectSwarmServiceCommand(value.Platform.Address, value.DockerServiceId),
            cancellationToken);
        if (!inspected.IsSuccess(out var service, out error))
            return Result.Failure<SwarmServiceResult>(error!);
        return string.Equals(service.Id, value.DockerServiceId, StringComparison.Ordinal)
            ? Result.Success(service)
            : Result.Failure<SwarmServiceResult>(new InternalServerError(
                "Docker returned a different Service than the one requested."));
    }
}

internal sealed class GetManagedSwarmServiceLogsHandler(
    IUnitOfWork unitOfWork,
    IConnectorFactory<ISwarmConnector> connectorFactory,
    IUserContextAccessor userContext)
    : IQueryHandler<GetManagedSwarmServiceLogs, Result<SwarmLogsResult>>
{
    public async ValueTask<Result<SwarmLogsResult>> Handle(
        GetManagedSwarmServiceLogs query,
        CancellationToken cancellationToken)
    {
        if (query.Tail is < 1 or > SwarmInventoryLimits.MaximumLogLines)
            return Result.Failure<SwarmLogsResult>(new BadRequestError(
                $"Tail must be between 1 and {SwarmInventoryLimits.MaximumLogLines}."));

        var context = await ManagedSwarmServiceRuntime.LoadAsync(
            query.Id, unitOfWork, userContext, cancellationToken);
        if (!context.IsSuccess(out var value, out var error))
            return Result.Failure<SwarmLogsResult>(error!);

        return await connectorFactory.GetConnector(value.Platform.ConnectorType).GetServiceLogsAsync(
            new GetSwarmServiceLogsCommand(value.Platform.Address, value.DockerServiceId, query.Tail),
            cancellationToken);
    }
}

internal static class ManagedSwarmServiceRuntime
{
    internal sealed record Context(Platform Platform, string DockerServiceId);

    public static async Task<Result<Context>> LoadAsync(
        Guid id,
        IUnitOfWork unitOfWork,
        IUserContextAccessor userContext,
        CancellationToken cancellationToken)
    {
        var service = await unitOfWork.SwarmServices.GetAsync(id, cancellationToken);
        if (service is null)
            return Result.Failure<Context>(new NotFoundError("The managed Swarm Service does not exist."));
        if (service.DockerServiceId is null)
            return Result.Failure<Context>(new ConflictError("The Service has not been deployed yet."));

        var platform = await unitOfWork.Platforms.GetByIdAsync(service.PlatformId, cancellationToken);
        if (platform is null
            || (!userContext.Current.IsAdmin
                && !await unitOfWork.Platforms.CanAccessAsync(
                    userContext.Current.UserId, service.PlatformId, cancellationToken)))
            return Result.Failure<Context>(new NotFoundError("The Docker Swarm platform does not exist or is not accessible."));
        if (platform.PlatformDescriptor is not DockerSwarmPlatformDescriptor)
            return Result.Failure<Context>(new BadRequestError("Managed Services require a Docker Swarm platform."));
        if (platform.Status != PlatformStatus.Online)
            return Result.Failure<Context>(new ConflictError("Platform is offline."));

        return Result.Success(new Context(platform, service.DockerServiceId));
    }
}
