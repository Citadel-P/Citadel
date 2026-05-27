using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Registries;
using Hosting.Common;
using Hosting.Common.Abstraction;
using LightResults;
using Mediator;

namespace Application.Features.Registries.Queries;

public sealed record GetAllRegistries(bool? IncludeDisabled) : IQuery<Result<IEnumerable<Registry>>>;

internal sealed class GetAllRegistriesHandler(IUnitOfWork unitOfWork, IUserContextAccessor userContextAccessor) : IQueryHandler<GetAllRegistries, Result<IEnumerable<Registry>>>
{
    public async ValueTask<Result<IEnumerable<Registry>>> Handle(GetAllRegistries query, CancellationToken cancellationToken)
    {
        var user = userContextAccessor.Current;
        var registries = user is not null && !user.IsAdmin
            ? await unitOfWork.Registries.GetAuthorizedAsync(user.UserId, ResourceType.Registry, PermissionLevel.Read, SpecificPermission.None, cancellationToken)
            : await unitOfWork.Registries.GetAllAsync(cancellationToken);

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
