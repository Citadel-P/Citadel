using Citadel.SourceGen;
using Domain.Contracts.Interfaces;
using Hosting.Common;
using Hosting.Common.Extensions;
using Hosting.Common.Pipelines.Interfaces;
using LightResults;
using Mediator;
using System.Security.Claims;

namespace Application.Services;

internal class PermissionService(IUnitOfWork uow) : IPermissionService
{
    public  Task<Result> EnforceAsync(IMessage message, ClaimsPrincipal user, CancellationToken cancellationToken = default)
        => PermissionPipeline.Enforce(
            message,
            user.GetUserId(),
            this,
            cancellationToken
        );
    
    public Task<bool> HasPermissionAsync(Guid userId, ResourceType resourceType, ResourceAction action, Guid? resourceId, CancellationToken ct)
        => uow.Users.HasPermissionAsync(userId, resourceType, action, resourceId, ct);
}
