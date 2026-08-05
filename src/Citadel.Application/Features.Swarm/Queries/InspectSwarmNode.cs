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
public sealed record InspectSwarmNode(Guid PlatformId, string NodeId) : IQuery<Result<SwarmNodeResult>>
{
    internal sealed class Validator : AbstractValidator<InspectSwarmNode>
    {
        public Validator()
        {
            RuleFor(query => query.PlatformId).NotEmpty();
            RuleFor(query => query.NodeId).NotEmpty().MaximumLength(255);
        }
    }
}

internal sealed class InspectSwarmNodeHandler(
    IUnitOfWork unitOfWork,
    IConnectorFactory<ISwarmConnector> connectorFactory)
    : IQueryHandler<InspectSwarmNode, Result<SwarmNodeResult>>
{
    public async ValueTask<Result<SwarmNodeResult>> Handle(
        InspectSwarmNode query,
        CancellationToken cancellationToken)
    {
        var platform = await unitOfWork.Platforms.GetByIdAsync(query.PlatformId, cancellationToken);
        if (platform is null)
            return Result.Failure<SwarmNodeResult>(new NotFoundError("Platform does not exist."));
        if (platform.PlatformDescriptor is not DockerSwarmPlatformDescriptor)
            return Result.Failure<SwarmNodeResult>(new BadRequestError("Node inspection is only available for Docker Swarm platforms."));
        if (platform.Status != PlatformStatus.Online)
            return Result.Failure<SwarmNodeResult>(new ConflictError("Platform is offline."));
        if (await unitOfWork.Swarm.GetNodeAsync(query.PlatformId, query.NodeId, cancellationToken) is null)
            return Result.Failure<SwarmNodeResult>(new NotFoundError("Swarm node does not exist."));

        var inspected = await connectorFactory.GetConnector(platform.ConnectorType).InspectNodeAsync(
            new InspectSwarmNodeCommand(platform.Address, query.NodeId),
            cancellationToken);
        if (!inspected.IsSuccess(out var node, out var error))
            return Result.Failure<SwarmNodeResult>(error!);
        if (!string.Equals(node.Id, query.NodeId, StringComparison.OrdinalIgnoreCase))
        {
            return Result.Failure<SwarmNodeResult>(
                new InternalServerError("Docker returned a different node than the one requested."));
        }

        return Result.Success(node);
    }
}
