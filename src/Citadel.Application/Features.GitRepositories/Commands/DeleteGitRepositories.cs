using Domain.Contracts.Interfaces;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.GitRepositories.Commands;

public sealed record DeleteGitRepositories(IEnumerable<Guid> Ids) : ICommand<Result>;

internal sealed class DeleteGitRepositoriesHandler(IUnitOfWork unitOfWork) : ICommandHandler<DeleteGitRepositories, Result>
{
    public async ValueTask<Result> Handle(DeleteGitRepositories command, CancellationToken cancellationToken)
    {
        var toDelete = await unitOfWork.GitRepositories.GetAllAsync(command.Ids, cancellationToken);
        if (toDelete is null || !toDelete.Any())
            return Result.Failure(new NotFoundError("No git repositories found matching the provided IDs for deletion."));

        var result = await unitOfWork.GitRepositories.RemoveRangeAsync(command.Ids, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        return result > 0
            ? Result.Success()
            : Result.Failure(new NotFoundError("No git repositories found matching the provided IDs for deletion."));
    }
}
