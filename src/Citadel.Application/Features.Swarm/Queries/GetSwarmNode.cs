using Application.TaskJobs;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Platforms;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Swarm.Queries;

[RequirePermission(ResourceType.Platform, PermissionLevel.Read)]
public sealed record GetSwarmNode(Guid PlatformId, string NodeId) : IQuery<Result<SwarmNodeProjection>>
{
    internal sealed class Validator : AbstractValidator<GetSwarmNode>
    {
        public Validator()
        {
            RuleFor(query => query.PlatformId).NotEmpty();
            RuleFor(query => query.NodeId).NotEmpty();
        }
    }
}

internal sealed class GetSwarmNodeHandler(
    IUnitOfWork unitOfWork,
    ISwarmReconciliationCoordinator reconciliationCoordinator)
    : IQueryHandler<GetSwarmNode, Result<SwarmNodeProjection>>
{
    public async ValueTask<Result<SwarmNodeProjection>> Handle(
        GetSwarmNode query,
        CancellationToken cancellationToken)
    {
        var platformError = await SwarmQuery.ValidateAndEnsureInitializedAsync(
            unitOfWork,
            reconciliationCoordinator,
            query.PlatformId,
            cancellationToken);
        if (platformError is not null)
            return Result.Failure<SwarmNodeProjection>(platformError);

        var node = await unitOfWork.Swarm.GetNodeAsync(query.PlatformId, query.NodeId, cancellationToken);
        return node ?? Result.Failure<SwarmNodeProjection>(new NotFoundError("Swarm node does not exist."));
    }
}
