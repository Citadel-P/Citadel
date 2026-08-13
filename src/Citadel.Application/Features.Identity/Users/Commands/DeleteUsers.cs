using Application.Services.Identity;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Activities;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Identity.Users.Commands;

[RequirePermission(ResourceType.User, PermissionLevel.Execute)]
public sealed record DeleteUsers(IEnumerable<Guid> Ids) : ICommand<Result>, IAdministratorRequest
{
    internal sealed class Validator : AbstractValidator<DeleteUsers>
    {
        public Validator()
            => RuleFor(command => command.Ids).NotNull().NotEmpty();
    }
}

internal sealed class DeleteUsersHandler(
    IUnitOfWork unitOfWork,
    IActorScopeEvictor evictor,
    IAdministratorGuard administratorGuard,
    IUserContextAccessor userContext) : ICommandHandler<DeleteUsers, Result>
{
    public async ValueTask<Result> Handle(DeleteUsers command, CancellationToken cancellationToken)
    {
        var users = await unitOfWork.Users.GetAllAsync(command.Ids, cancellationToken);
        if (users is null || !users.Any())
            return Result.Failure(new NotFoundError("No users found matching the provided IDs."));

        var userArray = users.ToArray();
        var snapshots = new List<(Domain.Entities.Identity.User User, UserActivitySnapshot Snapshot)>(userArray.Length);
        foreach (var user in userArray)
        {
            var snapshot = await IdentityActivity.CaptureUserAsync(unitOfWork, user.Id, cancellationToken);
            if (snapshot is not null)
                snapshots.Add((user, snapshot));
        }

        var userIds = userArray.Select(static user => user.Id).ToArray();
        await evictor.EvictUsers(userIds, cancellationToken);
        await unitOfWork.Users.RemoveRangeAsync(userIds, cancellationToken);

        var guardResult = await administratorGuard.EnsureAdministratorRemainsAsync(cancellationToken);
        if (guardResult.IsFailure(out var guardError))
            return Result.Failure(guardError);

        foreach (var (user, snapshot) in snapshots)
        {
            await unitOfWork.ActivityEventRepository.AddAsync(
                IdentityActivity.Create(
                    user.Id,
                    user.Name,
                    userContext.Current.ActorId,
                    ActivityEventType.UserDeleted,
                    new UserDeleted(snapshot)),
                cancellationToken);
        }

        await unitOfWork.CommitAsync(cancellationToken);
        return Result.Success();
    }
}
