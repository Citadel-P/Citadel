using Domain.Contracts.Interfaces;
using Hosting.Common.ErrorTypes;
using LightResults;

namespace Application.Services.Identity;

public interface IActorRoleService
{
    Task<Result> AssignRoleAsync(Guid actorId, Guid roleId, CancellationToken cancellationToken);
    Task<Result> RemoveRoleAsync(Guid actorId, Guid roleId, CancellationToken cancellationToken);
}

internal sealed class ActorRoleService(IUnitOfWork unitOfWork) : IActorRoleService
{
    public async Task<Result> AssignRoleAsync(Guid actorId, Guid roleId, CancellationToken cancellationToken)
    {
        var role = await unitOfWork.Roles.GetAsync(roleId, cancellationToken);
        if (role is null)
            return Result.Failure(new NotFoundError("The provided role does not exist"));

        var rows = await unitOfWork.Roles.AddActorRoleAsync(actorId, roleId, cancellationToken);
        if (rows == 0)
            return Result.Failure(new ConflictError("The role is already assigned to the actor"));

        await unitOfWork.CommitAsync(cancellationToken);
        return Result.Success();
    }

    public async Task<Result> RemoveRoleAsync(Guid actorId, Guid roleId, CancellationToken cancellationToken)
    {
        var role = await unitOfWork.Roles.GetAsync(roleId, cancellationToken);
        if (role is null)
            return Result.Failure(new NotFoundError("The provided role does not exist"));

        var rows = await unitOfWork.Roles.RemoveActorRoleAsync(actorId, roleId, cancellationToken);
        if (rows == 0)
            return Result.Failure(new NotFoundError("The role is not assigned to the actor"));

        await unitOfWork.CommitAsync(cancellationToken);
        return Result.Success();
    }
}
