using Application.Services;
using Hosting.Common;
using Domain.Contracts.Resources.Deployments;
using Hosting.Common.Attributes;
using Hosting.Common.Extensions;
using Mediator;
using Microsoft.AspNetCore.Http;
using System.Runtime.CompilerServices;
using System.Security.Claims;

namespace Application.Features.Deployments.Commands;

[RequirePermission(ResourceType.Deployment, PermissionLevel.Read, SpecificPermission.Apply)]
public sealed record ApplyDeployment(Guid Id, bool? Recreate = false) : IStreamCommand<DeploymentStreamItem>;

internal sealed class ApplyDeploymentHandler(IApplyDeploymentService deploymentApplyService, IHttpContextAccessor httpContextAccessor)
    : IStreamCommandHandler<ApplyDeployment, DeploymentStreamItem>
{
    public async IAsyncEnumerable<DeploymentStreamItem> Handle(ApplyDeployment command, [EnumeratorCancellation] CancellationToken ct)
    {
        var actorId = httpContextAccessor.HttpContext?.User?.GetActorId()
           ?? throw new ArgumentNullException($"{nameof(ClaimsPrincipal)} is missing");

        await foreach (var item in deploymentApplyService.ApplyAsync(command.Id, actorId, command.Recreate == true, ct))
        {
            yield return item;
        }
    }
}


