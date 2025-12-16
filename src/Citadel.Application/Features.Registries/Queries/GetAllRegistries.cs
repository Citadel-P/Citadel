using Domain.Contracts.Interfaces;
using Domain.Entities;
using LightResults;
using Mediator;

namespace Application.Features.Registries.Queries;

public sealed record GetAllRegistries : IQuery<Result<IEnumerable<Registry>>>;

internal sealed class GetAllRegistriesHandler(IUnitOfWork unitOfWork) : IQueryHandler<GetAllRegistries, Result<IEnumerable<Registry>>>
{
    public async ValueTask<Result<IEnumerable<Registry>>> Handle(GetAllRegistries query, CancellationToken cancellationToken)
    {
        var registries = await unitOfWork.Registries.GetAllAsync(cancellationToken) ?? [];
        var allRegistries = registries.OrderByDescending(s => s.CreatedAt);

        return Result.Success<IEnumerable<Registry>>(allRegistries);
    }
}
