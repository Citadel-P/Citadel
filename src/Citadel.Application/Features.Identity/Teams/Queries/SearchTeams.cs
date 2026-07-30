using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Identity;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.Attributes;
using LightResults;
using Mediator;

namespace Application.Features.Identity.Teams.Queries;

[RequirePermission(ResourceType.Team, PermissionLevel.Read)]
public sealed record SearchTeams(string Query, int Limit = 20) : IQuery<Result<IEnumerable<TeamSearchItem>>>, IAdministratorRequest
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

internal sealed class SearchTeamsHandler(IUnitOfWork unitOfWork, IUserContextAccessor userContextAccessor) : IQueryHandler<SearchTeams, Result<IEnumerable<TeamSearchItem>>>
{
    public async ValueTask<Result<IEnumerable<TeamSearchItem>>> Handle(SearchTeams query, CancellationToken cancellationToken)
    {
        var search = query.Query.Trim();

        var user = userContextAccessor.Current;
        var items = user is not null && !user.IsAdmin
            ? await unitOfWork.Teams.SearchAuthorizedAsync(user.UserId, ResourceType.Team, PermissionLevel.Read, SpecificPermission.None, search, query.Limit, cancellationToken)
            : await unitOfWork.Teams.SearchAsync(search, query.Limit, cancellationToken);

        return Result.Success(items);
    }
}
