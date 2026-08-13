using Application.Services.Identity;
using Application.Services.Licensing;
using Application.Services;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Identity;
using Domain.Entities.Identity;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Identity.ServiceAccounts;

[RequirePermission(ResourceType.ServiceAccount, PermissionLevel.Write, ResourceIdProperty = nameof(AddServiceAccountRole.Id))]
public sealed record AddServiceAccountRole(Guid Id, Guid RoleId)
    : ICommand<Result<ServiceAccountDetails>>, IAdministratorRequest
{
    internal sealed class Validator : AbstractValidator<AddServiceAccountRole>
    {
        public Validator()
        {
            RuleFor(command => command.Id).NotEmpty();
            RuleFor(command => command.RoleId).NotEmpty();
        }
    }
}

internal sealed class AddServiceAccountRoleHandler(
    IUnitOfWork unitOfWork,
    IUserContextAccessor userContext,
    IActorScopeEvictor evictor,
    ILicenseEntitlementService entitlementService)
    : ICommandHandler<AddServiceAccountRole, Result<ServiceAccountDetails>>
{
    public async ValueTask<Result<ServiceAccountDetails>> Handle(
        AddServiceAccountRole command,
        CancellationToken cancellationToken)
    {
        var state = await ServiceAccountMutationState.LoadLockedAsync(unitOfWork, command.Id, cancellationToken);
        if (state is null)
            return Result.Failure<ServiceAccountDetails>(new NotFoundError("The Service Account does not exist."));
        var role = await unitOfWork.Roles.GetAsync(command.RoleId, cancellationToken);
        if (role is null)
            return Result.Failure<ServiceAccountDetails>(new NotFoundError("The Role does not exist."));

        var customAccess = await entitlementService.EnsureEnabledAsync(
            LicenseCapability.CustomAccessControl,
            cancellationToken);
        if (customAccess.IsFailure(out var customAccessError))
            return Result.Failure<ServiceAccountDetails>(customAccessError);

        var oldSnapshot = state.Snapshot();
        if (await unitOfWork.Roles.AddActorRoleAsync(state.Actor.Id, command.RoleId, cancellationToken) == 0)
            return Result.Failure<ServiceAccountDetails>(new ConflictError("The Role is already assigned to the Service Account."));
        return await ServiceAccountMutationState.CompleteAsync(
            unitOfWork,
            userContext,
            evictor,
            state,
            oldSnapshot,
            cancellationToken,
            roleIds: [.. state.RoleIds, command.RoleId]);
    }
}

[RequirePermission(ResourceType.ServiceAccount, PermissionLevel.Write, ResourceIdProperty = nameof(RemoveServiceAccountRole.Id))]
public sealed record RemoveServiceAccountRole(Guid Id, Guid RoleId)
    : ICommand<Result<ServiceAccountDetails>>, IAdministratorRequest
{
    internal sealed class Validator : AbstractValidator<RemoveServiceAccountRole>
    {
        public Validator()
        {
            RuleFor(command => command.Id).NotEmpty();
            RuleFor(command => command.RoleId).NotEmpty();
        }
    }
}

internal sealed class RemoveServiceAccountRoleHandler(
    IUnitOfWork unitOfWork,
    IUserContextAccessor userContext,
    IActorScopeEvictor evictor)
    : ICommandHandler<RemoveServiceAccountRole, Result<ServiceAccountDetails>>
{
    public async ValueTask<Result<ServiceAccountDetails>> Handle(
        RemoveServiceAccountRole command,
        CancellationToken cancellationToken)
    {
        var state = await ServiceAccountMutationState.LoadLockedAsync(unitOfWork, command.Id, cancellationToken);
        if (state is null)
            return Result.Failure<ServiceAccountDetails>(new NotFoundError("The Service Account does not exist."));
        var oldSnapshot = state.Snapshot();
        if (await unitOfWork.Roles.RemoveActorRoleAsync(state.Actor.Id, command.RoleId, cancellationToken) == 0)
            return Result.Failure<ServiceAccountDetails>(new NotFoundError("The Role is not assigned to the Service Account."));
        return await ServiceAccountMutationState.CompleteAsync(
            unitOfWork,
            userContext,
            evictor,
            state,
            oldSnapshot,
            cancellationToken,
            roleIds: state.RoleIds.Where(roleId => roleId != command.RoleId).ToArray());
    }
}

[RequirePermission(ResourceType.ServiceAccount, PermissionLevel.Write, ResourceIdProperty = nameof(AddServiceAccountResourceAccess.Id))]
public sealed record AddServiceAccountResourceAccess(
    Guid Id,
    ResourceType ResourceType,
    Guid ResourceId,
    PermissionLevel PermissionLevel,
    IEnumerable<SpecificPermission>? SpecificPermissions)
    : ICommand<Result<ServiceAccountDetails>>, IAdministratorRequest
{
    internal sealed class Validator : AbstractValidator<AddServiceAccountResourceAccess>
    {
        public Validator()
        {
            RuleFor(command => command.Id).NotEmpty();
            RuleFor(command => command.ResourceId).NotEmpty();
            RuleFor(command => command.ResourceType).IsInEnum();
            RuleFor(command => command.PermissionLevel).IsInEnum();
            RuleForEach(command => command.SpecificPermissions).IsInEnum();
            RuleFor(command => command)
                .Must(command => PermissionMatrix.IsAllowed(
                    command.ResourceType,
                    command.PermissionLevel,
                    command.SpecificPermissions))
                .WithMessage("Invalid permission combination in resource access.");
        }
    }
}

internal sealed class AddServiceAccountResourceAccessHandler(
    IUnitOfWork unitOfWork,
    IUserContextAccessor userContext,
    IActorScopeEvictor evictor,
    ILicenseEntitlementService entitlementService)
    : ICommandHandler<AddServiceAccountResourceAccess, Result<ServiceAccountDetails>>
{
    public async ValueTask<Result<ServiceAccountDetails>> Handle(
        AddServiceAccountResourceAccess command,
        CancellationToken cancellationToken)
    {
        var state = await ServiceAccountMutationState.LoadLockedAsync(unitOfWork, command.Id, cancellationToken);
        if (state is null)
            return Result.Failure<ServiceAccountDetails>(new NotFoundError("The Service Account does not exist."));
        var entitlement = await entitlementService.EnsureEnabledAsync(
            LicenseCapability.CustomAccessControl,
            cancellationToken);
        if (entitlement.IsFailure(out var entitlementError))
            return Result.Failure<ServiceAccountDetails>(entitlementError);

        var access = ResourceAccess.Create(
            command.ResourceType,
            command.ResourceId,
            state.Actor.Id,
            command.PermissionLevel,
            command.SpecificPermissions);
        var oldSnapshot = state.Snapshot();
        if (await unitOfWork.ResourceAccesses.AddAsync(access, cancellationToken) == 0)
            return Result.Failure<ServiceAccountDetails>(new ConflictError("The resource access is already assigned to the Service Account."));
        return await ServiceAccountMutationState.CompleteAsync(
            unitOfWork,
            userContext,
            evictor,
            state,
            oldSnapshot,
            cancellationToken,
            resourceAccesses: [.. state.ResourceAccesses, access]);
    }
}

[RequirePermission(ResourceType.ServiceAccount, PermissionLevel.Write, ResourceIdProperty = nameof(RemoveServiceAccountResourceAccess.Id))]
public sealed record RemoveServiceAccountResourceAccess(Guid Id, Guid ResourceAccessId)
    : ICommand<Result<ServiceAccountDetails>>, IAdministratorRequest
{
    internal sealed class Validator : AbstractValidator<RemoveServiceAccountResourceAccess>
    {
        public Validator()
        {
            RuleFor(command => command.Id).NotEmpty();
            RuleFor(command => command.ResourceAccessId).NotEmpty();
        }
    }
}

internal sealed class RemoveServiceAccountResourceAccessHandler(
    IUnitOfWork unitOfWork,
    IUserContextAccessor userContext,
    IActorScopeEvictor evictor)
    : ICommandHandler<RemoveServiceAccountResourceAccess, Result<ServiceAccountDetails>>
{
    public async ValueTask<Result<ServiceAccountDetails>> Handle(
        RemoveServiceAccountResourceAccess command,
        CancellationToken cancellationToken)
    {
        var state = await ServiceAccountMutationState.LoadLockedAsync(unitOfWork, command.Id, cancellationToken);
        if (state is null)
            return Result.Failure<ServiceAccountDetails>(new NotFoundError("The Service Account does not exist."));
        var access = state.ResourceAccesses.FirstOrDefault(access => access.Id == command.ResourceAccessId);
        if (access is null)
            return Result.Failure<ServiceAccountDetails>(new NotFoundError("The resource access is not assigned to the Service Account."));

        var oldSnapshot = state.Snapshot();
        if (await unitOfWork.ResourceAccesses.RemoveByIdAsync(
                state.Actor.Id,
                command.ResourceAccessId,
                cancellationToken) == 0)
            return Result.Failure<ServiceAccountDetails>(new NotFoundError("The resource access is not assigned to the Service Account."));
        return await ServiceAccountMutationState.CompleteAsync(
            unitOfWork,
            userContext,
            evictor,
            state,
            oldSnapshot,
            cancellationToken,
            resourceAccesses: state.ResourceAccesses.Where(item => item.Id != command.ResourceAccessId).ToArray());
    }
}
