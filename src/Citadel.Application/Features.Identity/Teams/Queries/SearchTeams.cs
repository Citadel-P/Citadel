using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Identity;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.Extensions;
using LightResults;
using Mediator;
using Microsoft.AspNetCore.Http;

namespace Application.Features.Identity.Teams.Queries;

[RequirePermission(ResourceType.Team, ResourceAction.View)]
public sealed record SearchTeams(string Query, int Limit = 20) : IQuery<Result<IEnumerable<TeamSearchItem>>>
{
    internal sealed class Validator : AbstractValidator<SearchTeams>
    {
        public Validator()
        {
            RuleFor(x => x.Query)
                .NotEmpty()
                .Must(value => !string.IsNullOrWhiteSpace(value))
                .MinimumLength(2)
                .MaximumLength(140);

            RuleFor(x => x.Limit)
                .GreaterThan(0)
                .LessThanOrEqualTo(50);
        }
    }
}

internal sealed class SearchTeamsHandler(IUnitOfWork unitOfWork, IHttpContextAccessor httpContextAccessor) : IQueryHandler<SearchTeams, Result<IEnumerable<TeamSearchItem>>>
{
    public async ValueTask<Result<IEnumerable<TeamSearchItem>>> Handle(SearchTeams query, CancellationToken cancellationToken)
    {
        var search = query.Query.Trim();

        var user = httpContextAccessor.HttpContext?.User;
        var items = user is not null && !user.IsAdmin()
            ? await unitOfWork.Teams.SearchAuthorizedAsync(user.GetUserId(), ResourceType.Team, ResourceAction.View, search, query.Limit, cancellationToken)
            : await unitOfWork.Teams.SearchAsync(search, query.Limit, cancellationToken);

        return Result.Success(items);
    }
}