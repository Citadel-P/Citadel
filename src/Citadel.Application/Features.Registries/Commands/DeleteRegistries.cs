using Domain.Contracts.Interfaces;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Registries.Commands;

public sealed record DeleteRegistries(IEnumerable<Guid> Ids) : ICommand<Result>;

internal class DeleteRegistriesHandler(IUnitOfWork unitOfWork) : ICommandHandler<DeleteRegistries, Result>
{
    public async ValueTask<Result> Handle(DeleteRegistries command, CancellationToken cancellationToken)
    {
        var result = await unitOfWork.Registries.RemoveRangeAsync(command.Ids, cancellationToken);
        await unitOfWork.CommitAsync();

        return result > 0
            ? Result.Success()
            : Result.Failure(new NotFoundError("No registries matching the provided IDs were found for deletion"));
    }
}
 