using Hosting.Common.Extensions;
using Infrastructure;
using Infrastructure.Entities;
using Microsoft.AspNetCore.Authorization;

namespace Application.Permissions.Requirements;

internal class EditContainerRequirement : IAuthorizationRequirement
{
}

internal class EditContainerHandler : AuthorizationHandler<EditContainerRequirement, Container>
{
    protected override Task HandleRequirementAsync(AuthorizationHandlerContext context, EditContainerRequirement requirement, Container resource)
    {
        if (context.User.IsAdmin())
        {
            context.Succeed(requirement);
        }
        else
        {
            if (context.User.HasPermission(nameof(AppPermission.EditContainers)))
            {
                context.Succeed(requirement);
            }
            else
            {
                context.Fail(new AuthorizationFailureReason(this, "User does not have permission to edit containers."));
            }
        }

        return Task.CompletedTask;
    }
}