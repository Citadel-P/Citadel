using Infrastructure.Entities;
using Mediator;
using Microsoft.EntityFrameworkCore;
using Infrastructure.EntityFramework;
using LightResults;

namespace Application.Features.Registries.Queries;

public sealed record GetAllRegistries : IQuery<Result<IEnumerable<Registry>>>;

internal sealed class GetAllRegistriesHandler(ApplicationDbContext dbContext) : IQueryHandler<GetAllRegistries, Result<IEnumerable<Registry>>>
{
    public async ValueTask<Result<IEnumerable<Registry>>> Handle(GetAllRegistries query, CancellationToken cancellationToken)
    {
        var registries = new List<Registry>() 
        {
            Registry.DefaultRegistry()
        };
        registries.AddRange(await dbContext.Registries.AsNoTracking().ToListAsync(cancellationToken));
        return registries.OrderByDescending(s => s.Created).ToList();
    }
}
