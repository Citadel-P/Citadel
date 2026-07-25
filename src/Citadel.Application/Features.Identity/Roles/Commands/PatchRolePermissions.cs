using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Role;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using Hosting.Common.MergePatch;
using LightResults;
using Mediator;
using Application.Services.Identity;
using Application.Services.Licensing;

namespace Application.Features.Identity.Roles.Commands;

[RequirePermission(ResourceType.Role, PermissionLevel.Write)]
public sealed record PatchRolePermissions(Guid Id, JsonMergePatchDocument<PatchRolePermissionsModel> Patch) : ICommand<Result<RoleDetails>>
{
    internal sealed class Validator : PatchCommandValidator<PatchRolePermissions, PatchRolePermissionsModel>
    {
        public Validator()
            : base(
                patchSelector: x => x.Patch,
                jsonTypeInfo: RoleJsonContext.Default.PatchRolePermissionsModel,
                modelValidator: new PatchRolePermissionsModelValidator())
        {
        }
    }

    internal sealed class PatchRolePermissionsModelValidator : AbstractValidator<PatchRolePermissionsModel>
    {
        public PatchRolePermissionsModelValidator()
        {
            RuleFor(x => x.Permissions).NotNull();
            RuleForEach(x => x.Permissions).SetValidator(new PermissionInputValidator());
        }
    }

    internal sealed class PermissionInputValidator : AbstractValidator<PatchPermissionModel>
    {
        public PermissionInputValidator()
        {
            RuleFor(x => x.ResourceType).IsInEnum();
            RuleFor(x => x.PermissionLevel).IsInEnum();
            RuleForEach(x => x.SpecificPermissions).IsInEnum();
            RuleFor(x => x)
                .Must(p => PermissionMatrix.IsAllowed(p.ResourceType, p.PermissionLevel, p.SpecificPermissions))
                .WithMessage(p => $"Invalid permission: [{p.ResourceType}]-[{p.PermissionLevel}] with specifics [{string.Join(", ", p.SpecificPermissions ?? [])}] is not an allowed combination.");
        }
    }
}

internal sealed class PatchRolePermissionsHandler(
    IUnitOfWork unitOfWork,
    IActorScopeEvictor evictor,
    ILicenseEntitlementService entitlementService) : ICommandHandler<PatchRolePermissions, Result<RoleDetails>>
{
    public async ValueTask<Result<RoleDetails>> Handle(PatchRolePermissions command, CancellationToken cancellationToken)
    {
        var role = await unitOfWork.Roles.GetAsync(command.Id, cancellationToken);
        if (role is null)
            return Result.Failure<RoleDetails>(new NotFoundError("The provided role does not exist"));

        var current = new PatchRolePermissionsModel(role.Permissions.Select(x => new PatchPermissionModel(x.ResourceType, x.PermissionLevel, x.SpecificPermissions)));
        var patched = command.Patch.ApplyTo(current, RoleJsonContext.Default.PatchRolePermissionsModel);
        var permissions = patched.Permissions.Select(x => x.ToDomain(role.Id)).ToArray();

        if (role.RoleType == RoleType.Custom
            && LicenseAccessControlPolicy.ExpandsPermissions(role.Permissions, permissions))
        {
            var entitlement = await entitlementService.EnsureEnabledAsync(
                LicenseCapability.CustomAccessControl,
                cancellationToken);
            if (entitlement.IsFailure(out var entitlementError))
                return Result.Failure<RoleDetails>(entitlementError);
        }

        var setPermissionsResult = role.SetPermissions(permissions);
        if (setPermissionsResult.IsFailure(out var error))
            return Result.Failure<RoleDetails>(error);

        await unitOfWork.Roles.ReplacePermissionsAsync(role.Id, permissions, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        // Invalidate permission caches for all users affected by this role change
        await evictor.EvictPermissionsForRoleAsync(role.Id, cancellationToken);

        return new RoleDetails(role.Id, role.Name, role.RoleType, role.Permissions);
    }
}
