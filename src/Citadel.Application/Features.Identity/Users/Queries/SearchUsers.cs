using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Identity;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.Attributes;
using LightResults;
using Mediator;

namespace Application.Features.Identity.Users.Queries;

[RequirePermission(ResourceType.User, PermissionLevel.Read)]
public sealed record SearchUsers(string Query, int Limit = 20) : IQuery<Result<IEnumerable<UserSearchItem>>>, IAdministratorRequest
{
    internal sealed class Validator : AbstractValidator<SearchUsers>
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

internal sealed class SearchUsersHandler(IUnitOfWork unitOfWork, IUserContextAccessor userContextAccessor) : IQueryHandler<SearchUsers, Result<IEnumerable<UserSearchItem>>>
{
    public async ValueTask<Result<IEnumerable<UserSearchItem>>> Handle(SearchUsers query, CancellationToken cancellationToken)
    {
        var search = query.Query.Trim();

        var user = userContextAccessor.Current;
        var items = user is not null && !user.IsAdmin
            ? await unitOfWork.Users.SearchAuthorizedAsync(user.ActorId, ResourceType.User, PermissionLevel.Read, SpecificPermission.None, search, query.Limit, cancellationToken)
            : await unitOfWork.Users.SearchAsync(search, query.Limit, cancellationToken);

        return Result.Success(items);
    }
}
