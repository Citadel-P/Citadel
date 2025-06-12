using Hosting.Common.Extensions;
using Domain;
using Domain.Entities;
using Microsoft.AspNetCore.Authorization;

namespace Application.Permissions.Requirements;

internal class DeleteContainerRequirement : IAuthorizationRequirement
{
}

internal class DeleteContainerHandler : AuthorizationHandler<DeleteContainerRequirement, Container>
{
    protected override Task HandleRequirementAsync(AuthorizationHandlerContext context, DeleteContainerRequirement requirement, Container resource)
    {
        if (context.User.IsAdmin())
        {
            context.Succeed(requirement);
        }
        else
        {
            if (context.User.HasPermission(nameof(AppPermission.Container_Delete)))
            {
                context.Succeed(requirement);
            }
            else
            {
                context.Fail(new AuthorizationFailureReason(this, "User does not have permission to delete containers."));
            }
        }
        
        return Task.CompletedTask;
    }
}
