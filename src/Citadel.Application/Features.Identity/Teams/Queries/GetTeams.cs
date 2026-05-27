using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Identity;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.Models;
using LightResults;
using Mediator;

namespace Application.Features.Identity.Teams.Queries;

public sealed record GetTeams(string? Name = null, int Page = 1, int PageSize = 50) : IQuery<Result<PagedResult<TeamDetails>>>
{
    internal sealed class Validator : AbstractValidator<GetTeams>
    {
        public Validator()
        {
            RuleFor(x => x.Page).GreaterThan(0);
            RuleFor(x => x.Name).MaximumLength(140);
            RuleFor(x => x.PageSize).GreaterThan(0).LessThanOrEqualTo(500);
        }
    }
}

internal sealed class GetTeamsHandler(IUnitOfWork unitOfWork, IUserContextAccessor userContextAccessor) : IQueryHandler<GetTeams, Result<PagedResult<TeamDetails>>>
{
    public async ValueTask<Result<PagedResult<TeamDetails>>> Handle(GetTeams query, CancellationToken cancellationToken)
    {
        var user = userContextAccessor.Current;
        var pagedTeams = user is not null && !user.IsAdmin
            ? await unitOfWork.Teams.GetAuthorizedPagedAsync(user.UserId, ResourceType.Team, PermissionLevel.Read, SpecificPermission.None, query.Page, query.PageSize, query.Name, cancellationToken)
            : await unitOfWork.Teams.GetPagedAsync(query.Page, query.PageSize, query.Name, cancellationToken);

        return Result.Success(pagedTeams);
    }
}
