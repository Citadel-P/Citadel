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

[RequirePermission(ResourceType.Platform, PermissionLevel.Read, SpecificPermission.Inspect)]
public sealed record InspectSwarmService(Guid PlatformId, string ServiceId) : IQuery<Result<SwarmServiceResult>>
{
    internal sealed class Validator : AbstractValidator<InspectSwarmService>
    {
        public Validator()
        {
            RuleFor(query => query.PlatformId).NotEmpty();
            RuleFor(query => query.ServiceId).NotEmpty().MaximumLength(255);
        }
    }
}

internal sealed class InspectSwarmServiceHandler(
    IUnitOfWork unitOfWork,
    IConnectorFactory<ISwarmConnector> connectorFactory)
    : IQueryHandler<InspectSwarmService, Result<SwarmServiceResult>>
{
    public async ValueTask<Result<SwarmServiceResult>> Handle(
        InspectSwarmService query,
        CancellationToken cancellationToken)
    {
        var platform = await unitOfWork.Platforms.GetByIdAsync(query.PlatformId, cancellationToken);
        if (platform is null)
            return Result.Failure<SwarmServiceResult>(new NotFoundError("Platform does not exist."));
        if (platform.PlatformDescriptor is not DockerSwarmPlatformDescriptor)
            return Result.Failure<SwarmServiceResult>(new BadRequestError("Service inspection is only available for Docker Swarm platforms."));
        if (platform.Status != PlatformStatus.Online)
            return Result.Failure<SwarmServiceResult>(new ConflictError("Platform is offline."));
        if (await unitOfWork.Swarm.GetServiceAsync(query.PlatformId, query.ServiceId, cancellationToken) is null)
            return Result.Failure<SwarmServiceResult>(new NotFoundError("Swarm service does not exist."));

        var inspected = await connectorFactory.GetConnector(platform.ConnectorType).InspectServiceAsync(
            new InspectSwarmServiceCommand(platform.Address, query.ServiceId),
            cancellationToken);
        if (!inspected.IsSuccess(out var service, out var error))
            return Result.Failure<SwarmServiceResult>(error!);
        if (!string.Equals(service.Id, query.ServiceId, StringComparison.OrdinalIgnoreCase))
        {
            return Result.Failure<SwarmServiceResult>(
                new InternalServerError("Docker returned a different service than the one requested."));
        }

        return Result.Success(service);
    }
}
