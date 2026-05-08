using Domain.Contracts.Interfaces;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;
using Domain.Contracts.Resources.Role;

namespace Application.Features.Identity.Roles.Queries;

[RequirePermission(ResourceType.Role, PermissionLevel.Read)]
public sealed record GetRole(Guid Id) : IQuery<Result<RoleDetails>>;

internal sealed class GetRoleHandler(IUnitOfWork unitOfWork) : IQueryHandler<GetRole, Result<RoleDetails>>
{
    public async ValueTask<Result<RoleDetails>> Handle(GetRole query, CancellationToken cancellationToken)
    {
        var role = await unitOfWork.Roles.GetAsync(query.Id, cancellationToken);
        if (role is null)
            return Result.Failure<RoleDetails>(new NotFoundError($"Role with ID {query.Id} does not exist"));

        return Result.Success(new RoleDetails(role.Id, role.Name, role.RoleType, role.Permissions));
    }
}
