using Domain.Contracts.Interfaces;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Identity.Users.Commands;

[RequirePermission(ResourceType.User, ResourceAction.Delete)]
public sealed record DeleteUsers(IEnumerable<Guid> Ids) : ICommand<Result>;

internal sealed class DeleteUsersHandler(IUnitOfWork unitOfWork) : ICommandHandler<DeleteUsers, Result>
{
    public async ValueTask<Result> Handle(DeleteUsers command, CancellationToken cancellationToken)
    {
        var users = await unitOfWork.Users.GetAllAsync(command.Ids, cancellationToken);
        if (users is null || !users.Any())
            return Result.Failure(new NotFoundError("No users found matching the provided IDs."));

        await unitOfWork.Users.RemoveRangeAsync(command.Ids, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);
        return Result.Success();
    }
}
