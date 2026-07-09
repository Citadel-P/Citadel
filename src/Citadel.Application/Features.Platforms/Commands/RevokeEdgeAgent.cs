using Domain;
using Domain.Contracts.Interfaces;
using Hosting.Common;
using Hosting.Common.Attributes;
using LightResults;
using Mediator;

namespace Application.Features.Platforms.Commands;

[RequirePermission(ResourceType.Platform, PermissionLevel.Execute)]
public sealed record RevokeEdgeAgent(Guid PlatformId) : ICommand<Result>;

internal sealed class RevokeEdgeAgentHandler(IEdgeAgentManagementService edgeAgentManagementService)
    : ICommandHandler<RevokeEdgeAgent, Result>
{
    public ValueTask<Result> Handle(RevokeEdgeAgent command, CancellationToken cancellationToken)
        => new(edgeAgentManagementService.RevokeAsync(command.PlatformId, DateTime.UtcNow, cancellationToken));
}
