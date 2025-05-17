using Infrastructure;
using Infrastructure.Entities;
using Microsoft.AspNetCore.Authorization;

namespace Application.Permissions.Requirements;

internal class DeleteContainerRequirement : IAuthorizationRequirement
{
}

internal class DeleteContainerHandler : AuthorizationHandler<DeleteContainerRequirement, ContainerInfo>
{
    protected override Task HandleRequirementAsync(AuthorizationHandlerContext context, DeleteContainerRequirement requirement, ContainerInfo resource)
    {
        if (context.User.IsAdmin())
        {
            context.Succeed(requirement);
        }
        else
        {
            if (context.User.HasPermission(AppPermission.DeleteContainers))
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
