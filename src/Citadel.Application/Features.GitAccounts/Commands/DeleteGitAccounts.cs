using Hosting.Common;
using Domain.Contracts.Interfaces;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.GitAccounts.Commands;

[RequirePermission(ResourceType.GitAccount, PermissionLevel.Execute)]
public sealed record DeleteGitAccounts(IEnumerable<Guid> Ids) : ICommand<Result>;

internal sealed class DeleteGitAccountsHandler(IUnitOfWork unitOfWork) : ICommandHandler<DeleteGitAccounts, Result>
{
    public async ValueTask<Result> Handle(DeleteGitAccounts command, CancellationToken cancellationToken)
    {
        var toDelete = await unitOfWork.GitAccounts.GetAllAsync(command.Ids, cancellationToken);
        if (toDelete is null || !toDelete.Any())
            return Result.Failure(new NotFoundError("No git accounts found matching the provided IDs for deletion."));

        var result = await unitOfWork.GitAccounts.RemoveRangeAsync(command.Ids, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        return result > 0
            ? Result.Success()
            : Result.Failure(new NotFoundError("No git accounts found matching the provided IDs for deletion."));
    }
}
