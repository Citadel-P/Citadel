using Infrastructure.Entities;
using Infrastructure.EntityFramework;
using LightResults;
using Mediator;
using Microsoft.EntityFrameworkCore;

namespace Application.Features.Registries.Commands;

public sealed record DeleteRegistries(IEnumerable<Guid> Ids) : ICommand<Result<IEnumerable<Registry>>>;

internal class DeleteRegistriesHandler(ApplicationDbContext dbContext) : ICommandHandler<DeleteRegistries, Result<IEnumerable<Registry>>>
{
    public async ValueTask<Result<IEnumerable<Registry>>> Handle(DeleteRegistries command, CancellationToken cancellationToken)
    {
        var registries = await dbContext.Registries.Where(s => command.Ids.Contains(s.Id)).ToListAsync(cancellationToken);
        dbContext.RemoveRange(registries);
        await dbContext.SaveChangesAsync(cancellationToken);
        return Result.Success<IEnumerable<Registry>>(registries);
    }
}
 