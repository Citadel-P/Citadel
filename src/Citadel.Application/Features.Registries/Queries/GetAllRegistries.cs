using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Registries;
using LightResults;
using Mediator;

namespace Application.Features.Registries.Queries;

public sealed record GetAllRegistries(bool? IncludeDisabled) : IQuery<Result<IEnumerable<Registry>>>;

internal sealed class GetAllRegistriesHandler(IUnitOfWork unitOfWork) : IQueryHandler<GetAllRegistries, Result<IEnumerable<Registry>>>
{
    public async ValueTask<Result<IEnumerable<Registry>>> Handle(GetAllRegistries query, CancellationToken cancellationToken)
    {
        var registries = await unitOfWork.Registries.GetAllAsync(cancellationToken) ?? [];
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
