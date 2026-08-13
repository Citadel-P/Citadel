using Application.Services.Licensing;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Identity;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using Hosting.Common.Pipelines.Interfaces;
using LightResults;

namespace Application.Services;

internal interface IRunAsActorAuthorization
{
    Task<Result> EnsureAllowedAsync(Guid runAsActorId, CancellationToken cancellationToken);
    Task<Result<RunAsActorInfo>> ValidateExecutionAsync(Guid runAsActorId, CancellationToken cancellationToken);
}

internal sealed class RunAsActorAuthorization(
    IUnitOfWork unitOfWork,
    IUserContextAccessor userContextAccessor,
    IPermissionService permissionService,
    ILicenseEntitlementService entitlementService) : IRunAsActorAuthorization
{
    public async Task<Result> EnsureAllowedAsync(Guid runAsActorId, CancellationToken cancellationToken)
    {
        await unitOfWork.Actors.AcquireRunAsLockAsync(runAsActorId, cancellationToken);
        var targetResult = await ValidateExecutionAsync(runAsActorId, cancellationToken);
        if (!targetResult.IsSuccess(out var target, out var targetError))
            return Result.Failure(targetError);

        var caller = userContextAccessor.Current;
        if (caller.IsAdmin || caller.ActorId == runAsActorId)
            return Result.Success();

        if (target.Type != ActorType.ServiceAccount)
            return Result.Failure(new ForbiddenError("Only administrators can run operations as another user."));

        var permissions = await permissionService.ResolvePermissionsAsync(
            caller.ActorId,
            ResourceType.ServiceAccount,
            target.PrincipalId,
            cancellationToken);
        return permissions.Has(PermissionLevel.Read, SpecificPermission.Use)
            ? Result.Success()
            : Result.Failure(new ForbiddenError("Use permission is required for this Service Account."));
    }

    public async Task<Result<RunAsActorInfo>> ValidateExecutionAsync(
        Guid runAsActorId,
        CancellationToken cancellationToken)
    {
        var target = await unitOfWork.Actors.GetRunAsInfoAsync(runAsActorId, cancellationToken);
        if (target is null || !target.IsEnabled || target.IsArchived)
            return Result.Failure<RunAsActorInfo>(new BadRequestError("The run-as identity is unavailable or disabled."));

        if (target.Type == ActorType.ServiceAccount)
        {
            var entitlement = await entitlementService.EnsureEnabledAsync(
                LicenseCapability.CustomAccessControl,
                cancellationToken);
            if (entitlement.IsFailure(out var entitlementError))
                return Result.Failure<RunAsActorInfo>(entitlementError);
        }
        else if (target.Type != ActorType.User)
        {
            return Result.Failure<RunAsActorInfo>(new BadRequestError("The selected Actor cannot be used to run operations."));
        }

        return Result.Success(target);
    }
}
