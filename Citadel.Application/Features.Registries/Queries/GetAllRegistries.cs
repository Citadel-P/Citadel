using Infrastructure.Entities;
using Mediator;
using Microsoft.EntityFrameworkCore;
using Infrastructure.Services.Abstractions;
using Infrastructure.EntityFramework;

namespace Application.Features.Registries.Queries;

public sealed record GetAllRegistries : IQuery<IEnumerable<Registry>>;

internal sealed class GetAllRegistriesHandler(ApplicationDbContext dbContext) : IQueryHandler<GetAllRegistries, IEnumerable<Registry>>
{
    public async ValueTask<IEnumerable<Registry>> Handle(GetAllRegistries query, CancellationToken cancellationToken)
    {
        return await dbContext.Registries.ToListAsync(cancellationToken);
    }
}