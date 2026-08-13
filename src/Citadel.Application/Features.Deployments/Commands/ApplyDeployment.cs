using Application.Services;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Deployments;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.Attributes;
using Mediator;
using System.Runtime.CompilerServices;

namespace Application.Features.Deployments.Commands;

[RequirePermission(ResourceType.Deployment, PermissionLevel.Read, SpecificPermission.Apply)]
public sealed record ApplyDeployment(Guid Id, bool? Recreate = false) : IStreamCommand<DeploymentStreamItem>;

internal sealed class ApplyDeploymentHandler(
    IApplyDeploymentService deploymentApplyService,
    IUnitOfWork unitOfWork,
    IUserContextAccessor userContext)
    : IStreamCommandHandler<ApplyDeployment, DeploymentStreamItem>
{
    public async IAsyncEnumerable<DeploymentStreamItem> Handle(ApplyDeployment command, [EnumeratorCancellation] CancellationToken ct)
    {
        var user = userContext.Current;
        var actorId = user.ActorId;

        if (!user.IsAdmin)
        {
            var deployment = await unitOfWork.Deployments.GetAsync(command.Id, ct);
            if (deployment is not null
                && !await unitOfWork.Platforms.CanAccessAsync(user.ActorId, deployment.PlatformId, ct))
            {
                const string message = "The deployment platform does not exist or is not accessible.";
                yield return new DeploymentStreamItem(
                    ErrorMessage: message,
                    Error: new DeploymentApplyError(404, message));
                yield break;
            }
        }

        await foreach (var item in deploymentApplyService.ApplyAsync(command.Id, actorId, command.Recreate == true, ct))
        {
            yield return item;
        }
    }
}


