using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Identity;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.Extensions;
using Hosting.Common.Models;
using LightResults;
using Mediator;
using Microsoft.AspNetCore.Http;

namespace Application.Features.Identity.Users.Queries;

[RequirePermission(ResourceType.User, ResourceAction.View)]
public sealed record GetUsers(int Page = 1, int PageSize = 50, string? Name = null) : IQuery<Result<PagedResult<UserDetails>>>
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

internal sealed class GetUsersHandler(IUnitOfWork unitOfWork, IHttpContextAccessor httpContextAccessor) : IQueryHandler<GetUsers, Result<PagedResult<UserDetails>>>
{
    public async ValueTask<Result<PagedResult<UserDetails>>> Handle(GetUsers query, CancellationToken cancellationToken)
    {
        var user = httpContextAccessor.HttpContext?.User;
        var pagedUsers = user is not null && !user.IsAdmin()
            ? await unitOfWork.Users.GetAuthorizedPagedAsync(user.GetUserId(), ResourceType.User, ResourceAction.View, query.Page, query.PageSize, query.Name, cancellationToken)
            : await unitOfWork.Users.GetPagedAsync(query.Page, query.PageSize, query.Name, cancellationToken);

        return Result.Success(pagedUsers);
    }
}
