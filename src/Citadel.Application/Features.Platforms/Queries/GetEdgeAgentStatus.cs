using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Platforms;
using Hosting.Common;
using Hosting.Common.Attributes;
using LightResults;
using Mediator;

namespace Application.Features.Platforms.Queries;

[RequirePermission(ResourceType.Platform, PermissionLevel.Read)]
public sealed record GetEdgeAgentStatus(Guid PlatformId) : IQuery<Result<EdgeAgentStatusResult>>;

internal sealed class GetEdgeAgentStatusHandler(IEdgeAgentManagementService edgeAgentManagementService)
    : IQueryHandler<GetEdgeAgentStatus, Result<EdgeAgentStatusResult>>
{
    public ValueTask<Result<EdgeAgentStatusResult>> Handle(GetEdgeAgentStatus query, CancellationToken cancellationToken)
        => new(edgeAgentManagementService.GetStatusAsync(query.PlatformId, DateTime.UtcNow, cancellationToken));
}
