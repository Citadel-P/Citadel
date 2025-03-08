using Infrastructure.Entities;
using Mediator;
using LightResults;
using Microsoft.EntityFrameworkCore;
using Infrastructure.EntityFramework;
using Hosting.Common.ErrorTypes;

namespace Application.Features.Registries.Queries;

public sealed record GetRegistry(Guid Id) : IQuery<Result<Registry>>;
internal sealed class GetRegistryHandler(ApplicationDbContext dbContext) : IQueryHandler<GetRegistry, Result<Registry>>
{
    public async ValueTask<Result<Registry>> Handle(GetRegistry query, CancellationToken cancellationToken)
    {
        var registry = await dbContext.Registries.AsNoTracking().SingleOrDefaultAsync(s => s.Id == query.Id,  cancellationToken);
        return registry ?? Result.Failure<Registry>(new NotFoundError($"Registry with id {query.Id} does not exist"));
    }
}