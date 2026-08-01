using Hosting.Common;
using Domain.Contracts.Interfaces;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.GitAccounts.Commands;

[RequirePermission(ResourceType.GitAccount, PermissionLevel.Execute)]
public sealed record DeleteGitAccounts(IEnumerable<Guid> Ids) : ICommand<Result>;

internal sealed class DeleteGitAccountsHandler(
    IUnitOfWork unitOfWork,
    IGitCliRepository gitCliRepository) : ICommandHandler<DeleteGitAccounts, Result>
{
    public async ValueTask<Result> Handle(DeleteGitAccounts command, CancellationToken cancellationToken)
    {
        var requestedIds = command.Ids.Distinct().ToArray();
        var toDelete = (await unitOfWork.GitAccounts.GetAllAsync(requestedIds, cancellationToken) ?? [])
            .ToArray();
        if (requestedIds.Length == 0 || toDelete.Length != requestedIds.Length)
            return Result.Failure(new NotFoundError("One or more git accounts were not found."));

        var result = await unitOfWork.GitAccounts.RemoveRangeAsync(requestedIds, cancellationToken);
        if (result != requestedIds.Length)
        {
            await unitOfWork.RollbackAsync();
            return Result.Failure(new ConflictError(
                "The git-account set changed while deletion was in progress."));
        }

        await unitOfWork.CommitAsync(cancellationToken);

        foreach (var account in toDelete)
            gitCliRepository.RemoveCredentialFile(account.Id);

        return Result.Success();
    }
}
