using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Identity;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.Models;
using LightResults;
using Mediator;

namespace Application.Features.Identity.Users.Queries;

public sealed record GetUsers(int Page = 1, int PageSize = 50, string? Name = null) : IQuery<Result<PagedResult<UserDetails>>>, IAdministratorRequest
{
    internal sealed class Validator : AbstractValidator<GetUsers>
    {
        public Validator()
        {
            RuleFor(x => x.Page).GreaterThan(0);
            RuleFor(x => x.Name).MaximumLength(140);
            RuleFor(x => x.PageSize).GreaterThan(0).LessThanOrEqualTo(500);
        }
    }
}

internal sealed class GetUsersHandler(IUnitOfWork unitOfWork, IUserContextAccessor userContextAccessor) : IQueryHandler<GetUsers, Result<PagedResult<UserDetails>>>
{
    public async ValueTask<Result<PagedResult<UserDetails>>> Handle(GetUsers query, CancellationToken cancellationToken)
    {
        var user = userContextAccessor.Current;
        var pagedUsers = user is not null && !user.IsAdmin
            ? await unitOfWork.Users.GetAuthorizedPagedAsync(user.ActorId, ResourceType.User, PermissionLevel.Read, SpecificPermission.None, query.Page, query.PageSize, query.Name, cancellationToken)
            : await unitOfWork.Users.GetPagedAsync(query.Page, query.PageSize, query.Name, cancellationToken);

        return Result.Success(pagedUsers);
    }
}
