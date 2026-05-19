using Domain.Contracts.Interfaces;
using Domain.Entities.Identity;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;

namespace Application.Services.Identity;

public interface IActorRoleService
{
    Task<Result> AssignRoleAsync(Guid actorId, Guid roleId, CancellationToken cancellationToken);
    Task<Result> RemoveRoleAsync(Guid actorId, Guid roleId, CancellationToken cancellationToken);
}

public interface IActorResourceAccessService
{
    Task<Result> AddResourceAccessAsync(
        Guid actorId,
        ResourceType resourceType,
        Guid resourceId,
        PermissionLevel permissionLevel,
        IEnumerable<SpecificPermission>? specificPermissions,
        CancellationToken cancellationToken);
    Task<Result> RemoveResourceAccessAsync(
        Guid actorId,
        ResourceType resourceType,
        Guid resourceId,
        PermissionLevel permissionLevel,
        IEnumerable<SpecificPermission>? specificPermissions,
        CancellationToken cancellationToken);
}

internal sealed class ActorRoleService(IUnitOfWork unitOfWork, IActorScopeEvictor evictor) : IActorRoleService
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
        await evictor.EvictPermissionsForActorAsync(actorId, cancellationToken);
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
        await evictor.EvictPermissionsForActorAsync(actorId, cancellationToken);
        return Result.Success();
    }
}

internal sealed class ActorResourceAccessService(IUnitOfWork unitOfWork) : IActorResourceAccessService
{
    public async Task<Result> AddResourceAccessAsync(
        Guid actorId,
        ResourceType resourceType,
        Guid resourceId,
        PermissionLevel permissionLevel,
        IEnumerable<SpecificPermission>? specificPermissions,
        CancellationToken cancellationToken)
    {
        if (!PermissionMatrix.IsAllowed(resourceType, permissionLevel, specificPermissions))
            return Result.Failure(new ConflictError($"Invalid permission: [{resourceType}]-[{permissionLevel}] with specifics [{string.Join(", ", specificPermissions ?? [])}] is not an allowed combination."));

        var actor = await unitOfWork.Actors.GetById(actorId, cancellationToken);
        if (actor is null)
            return Result.Failure(new NotFoundError("The provided actor does not exist"));

        var resourceAccess = ResourceAccess.Create(resourceType, resourceId, actorId, permissionLevel, specificPermissions);
        var rows = await unitOfWork.ResourceAccesses.AddAsync(resourceAccess, cancellationToken);
        if (rows == 0)
            return Result.Failure(new ConflictError("The resource access is already assigned to the actor"));

        await unitOfWork.CommitAsync(cancellationToken);
        return Result.Success();
    }

    public async Task<Result> RemoveResourceAccessAsync(
        Guid actorId,
        ResourceType resourceType,
        Guid resourceId,
        PermissionLevel permissionLevel,
        IEnumerable<SpecificPermission>? specificPermissions,
        CancellationToken cancellationToken)
    {
        if (!PermissionMatrix.IsAllowed(resourceType, permissionLevel, specificPermissions))
            return Result.Failure(new ConflictError($"Invalid permission: [{resourceType}]-[{permissionLevel}] with specifics [{string.Join(", ", specificPermissions ?? [])}] is not an allowed combination."));

        var actor = await unitOfWork.Actors.GetById(actorId, cancellationToken);
        if (actor is null)
            return Result.Failure(new NotFoundError("The provided actor does not exist"));

        var rows = await unitOfWork.ResourceAccesses.RemoveAsync(actorId, resourceType, resourceId, permissionLevel, specificPermissions, cancellationToken);
        if (rows == 0)
            return Result.Failure(new NotFoundError("The resource access is not assigned to the actor"));

        await unitOfWork.CommitAsync(cancellationToken);
        return Result.Success();
    }
}
