using Domain.Contracts.Interfaces;
using Domain.Entities;
using LightResults;
using Mediator;
using Microsoft.EntityFrameworkCore;

namespace Application.Features.Registries.Commands;

public sealed record DeleteRegistries(IEnumerable<Guid> Ids) : ICommand<Result<IEnumerable<Registry>>>;

internal class DeleteRegistriesHandler(IUnitOfWork unitOfWork) : ICommandHandler<DeleteRegistries, Result<IEnumerable<Registry>>>
{
    public async ValueTask<Result<IEnumerable<Registry>>> Handle(DeleteRegistries command, CancellationToken cancellationToken)
    {
        var repo = unitOfWork.Registries;
        var registries = await repo.Query().AsNoTracking().Where(s => command.Ids.Contains(s.Id)).ToListAsync(cancellationToken);

        repo.RemoveRange(registries);
        await unitOfWork.SaveChangesAsync(cancellationToken);

        return registries;
    }
}
 