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
public sealed record GetSwarmNodes(Guid PlatformId) : IQuery<Result<IReadOnlyList<SwarmNodeProjection>>>
{
    internal sealed class Validator : AbstractValidator<GetSwarmNodes>
    {
        public Validator() => RuleFor(query => query.PlatformId).NotEmpty();
    }
}

internal sealed class GetSwarmNodesHandler(
    IUnitOfWork unitOfWork,
    ISwarmReconciliationCoordinator reconciliationCoordinator)
    : IQueryHandler<GetSwarmNodes, Result<IReadOnlyList<SwarmNodeProjection>>>
{
    public async ValueTask<Result<IReadOnlyList<SwarmNodeProjection>>> Handle(
        GetSwarmNodes query,
        CancellationToken cancellationToken)
    {
        var platformError = await SwarmQuery.ValidateAndEnsureInitializedAsync(
            unitOfWork,
            reconciliationCoordinator,
            query.PlatformId,
            cancellationToken);
        if (platformError is not null)
            return Result.Failure<IReadOnlyList<SwarmNodeProjection>>(platformError);

        return Result.Success(await unitOfWork.Swarm.GetNodesAsync(query.PlatformId, cancellationToken));
    }

    internal static async Task<IError?> ValidatePlatformAsync(
        IUnitOfWork unitOfWork,
        Guid platformId,
        CancellationToken cancellationToken)
    {
        var platform = await unitOfWork.Platforms.GetByIdAsync(platformId, cancellationToken);
        if (platform is null)
            return new NotFoundError("Platform does not exist.");
        return platform.PlatformDescriptor is DockerSwarmPlatformDescriptor
            ? null
            : new BadRequestError("Swarm nodes are only available for Docker Swarm platforms.");
    }
}
