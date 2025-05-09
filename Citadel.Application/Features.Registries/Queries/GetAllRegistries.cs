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
        var result = await dbContext.Registries.AsNoTracking().ToListAsync(cancellationToken);
        if (result.Count != 0)
        {
            registries.AddRange(result);
        }
        return registries.OrderByDescending(s => s.Created).ToList();
    }
}
