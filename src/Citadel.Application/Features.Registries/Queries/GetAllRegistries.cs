using Domain;
using Application.Features.Tags.Queries;
using Domain.Contracts.Interfaces;
using Domain.Entities.Registries;
using Hosting.Common;
using Hosting.Common.Abstraction;
using LightResults;
using Mediator;

namespace Application.Features.Registries.Queries;

public sealed record GetAllRegistries(bool? IncludeDisabled, IReadOnlyCollection<string>? Tags = null) : IQuery<Result<IEnumerable<Registry>>>;

internal sealed class GetAllRegistriesHandler(IUnitOfWork unitOfWork, IUserContextAccessor userContextAccessor) : IQueryHandler<GetAllRegistries, Result<IEnumerable<Registry>>>
{
    public async ValueTask<Result<IEnumerable<Registry>>> Handle(GetAllRegistries query, CancellationToken cancellationToken)
    {
        var tagFilter = await TagFilterResolver.ResolveAsync(unitOfWork, query.Tags, cancellationToken);
        if (tagFilter.NoMatch)
            return Result.Success<IEnumerable<Registry>>([]);

        var user = userContextAccessor.Current;
        var registries = user is not null && !user.IsAdmin
            ? await unitOfWork.Registries.GetAuthorizedAsync(user.ActorId, ResourceType.Registry, PermissionLevel.Read, SpecificPermission.None, cancellationToken, tagFilter.TagIds)
            : await unitOfWork.Registries.GetAllAsync(cancellationToken, tagFilter.TagIds);

        if (query.IncludeDisabled == true)
        {
            return Result.Success<IEnumerable<Registry>>(registries.OrderByDescending(s => s.CreatedAt));
        }
        else
        {
            return Result.Success(registries.OrderByDescending(s => s.CreatedAt).Where(r => r.Status != RegistryStatus.Disabled));
        }
    }
}
