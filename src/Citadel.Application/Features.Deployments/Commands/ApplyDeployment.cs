using Application.Services;
using Domain.Contracts.Resources.Deployments;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.Attributes;
using Mediator;
using System.Runtime.CompilerServices;

namespace Application.Features.Deployments.Commands;

[RequirePermission(ResourceType.Deployment, PermissionLevel.Read, SpecificPermission.Apply)]
public sealed record ApplyDeployment(Guid Id, bool? Recreate = false) : IStreamCommand<DeploymentStreamItem>;

internal sealed class ApplyDeploymentHandler(IApplyDeploymentService deploymentApplyService, IUserContextAccessor userContext)
    : IStreamCommandHandler<ApplyDeployment, DeploymentStreamItem>
{
    public async IAsyncEnumerable<DeploymentStreamItem> Handle(ApplyDeployment command, [EnumeratorCancellation] CancellationToken ct)
    {
        var actorId = userContext.Current.ActorId;
        await foreach (var item in deploymentApplyService.ApplyAsync(command.Id, actorId, command.Recreate == true, ct))
        {
            yield return item;
        }
    }
}


