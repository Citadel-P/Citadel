using Application.Services.Identity;
using Domain.Contracts.Interfaces;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Identity.Users.Commands;

[RequirePermission(ResourceType.User, PermissionLevel.Execute)]
public sealed record DeleteUsers(IEnumerable<Guid> Ids) : ICommand<Result>, IAdministratorRequest;

internal sealed class DeleteUsersHandler(
    IUnitOfWork unitOfWork,
    IActorScopeEvictor evictor,
    IAdministratorGuard administratorGuard) : ICommandHandler<DeleteUsers, Result>
{
    public async ValueTask<Result> Handle(DeleteUsers command, CancellationToken cancellationToken)
    {
        var users = await unitOfWork.Users.GetAllAsync(command.Ids, cancellationToken);
        if (users is null || !users.Any())
            return Result.Failure(new NotFoundError("No users found matching the provided IDs."));

        var userIds = users.Select(static user => user.Id).ToArray();
        await unitOfWork.Users.RemoveRangeAsync(userIds, cancellationToken);

        var guardResult = await administratorGuard.EnsureAdministratorRemainsAsync(cancellationToken);
        if (guardResult.IsFailure(out var guardError))
            return Result.Failure(guardError);

        await unitOfWork.CommitAsync(cancellationToken);
        await evictor.EvictUsers(userIds, cancellationToken);
        return Result.Success();
    }
}
