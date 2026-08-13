using Application.Features.Tags.Queries;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Swarm;
using Domain.Entities.Platforms;
using Domain.Entities.SwarmServices;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.SwarmServices.Queries;

public sealed record GetSwarmServices(IReadOnlyCollection<string>? Tags = null, Guid? PlatformId = null)
    : IQuery<Result<ManagedSwarmServicesResult>>;

public sealed record ManagedSwarmServicesResult(
    IReadOnlyList<SwarmService> Services,
    IReadOnlyDictionary<Guid, IReadOnlyList<SwarmTaskProjection>> TasksByServiceId);

[RequirePermission(ResourceType.SwarmService, PermissionLevel.Read, ResourceIdProperty = nameof(Id))]
public sealed record GetSwarmService(Guid Id) : IQuery<Result<SwarmService>>;

internal sealed class GetSwarmServicesHandler(
    IUnitOfWork unitOfWork,
    IUserContextAccessor userContext) : IQueryHandler<GetSwarmServices, Result<ManagedSwarmServicesResult>>
{
    public async ValueTask<Result<ManagedSwarmServicesResult>> Handle(
        GetSwarmServices query,
        CancellationToken cancellationToken)
    {
        var tagFilter = await TagFilterResolver.ResolveAsync(unitOfWork, query.Tags, cancellationToken);
        if (tagFilter.NoMatch)
            return Result.Success(new ManagedSwarmServicesResult([], new Dictionary<Guid, IReadOnlyList<SwarmTaskProjection>>()));

        var user = userContext.Current;
        var services = (user.IsAdmin || user.ActorId == Constants.SystemId
            ? await unitOfWork.SwarmServices.GetInfoAsync(
                cancellationToken,
                tagFilter.TagIds,
                query.PlatformId)
            : await unitOfWork.SwarmServices.GetAuthorizedInfoAsync(
                user.ActorId,
                PermissionLevel.Read,
                SpecificPermission.None,
                cancellationToken,
                tagFilter.TagIds,
                query.PlatformId)).ToArray();

        var tasksByServiceId = new Dictionary<Guid, IReadOnlyList<SwarmTaskProjection>>();
        foreach (var platformServices in services.GroupBy(static service => service.PlatformId))
        {
            var projections = (await unitOfWork.Swarm.GetServicesAsync(platformServices.Key, cancellationToken))
                .ToDictionary(static projection => projection.DockerServiceId, StringComparer.Ordinal);
            var tasks = (await unitOfWork.Swarm.GetTasksAsync(
                    platformServices.Key,
                    SwarmInventoryLimits.MaximumItems,
                    cancellationToken))
                .ToLookup(static task => task.DockerServiceId, StringComparer.Ordinal);
            foreach (var service in platformServices)
            {
                projections.TryGetValue(service.DockerServiceId ?? string.Empty, out var projection);
                service.ApplyObservation(projection);
                tasksByServiceId[service.Id] = service.DockerServiceId is null
                    ? []
                    : tasks[service.DockerServiceId].ToArray();
            }
        }

        return Result.Success(new ManagedSwarmServicesResult(services, tasksByServiceId));
    }
}

internal sealed class GetSwarmServiceHandler(
    IUnitOfWork unitOfWork,
    IUserContextAccessor userContext)
    : IQueryHandler<GetSwarmService, Result<SwarmService>>
{
    public async ValueTask<Result<SwarmService>> Handle(GetSwarmService query, CancellationToken cancellationToken)
    {
        var service = await unitOfWork.SwarmServices.GetAsync(query.Id, cancellationToken);
        if (service is null)
            return Result.Failure<SwarmService>(new NotFoundError("The managed Swarm Service does not exist."));

        if (!userContext.Current.IsAdmin
            && userContext.Current.ActorId != Constants.SystemId
            && !await unitOfWork.Platforms.CanAccessAsync(
                userContext.Current.ActorId, service.PlatformId, cancellationToken))
            return Result.Failure<SwarmService>(new NotFoundError("The managed Swarm Service does not exist."));

        service.ApplyObservation(service.DockerServiceId is null
            ? null
            : await unitOfWork.Swarm.GetServiceAsync(
                service.PlatformId, service.DockerServiceId, cancellationToken));
        return Result.Success(service);
    }
}
