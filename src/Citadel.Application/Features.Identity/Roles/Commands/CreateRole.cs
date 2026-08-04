using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Role;
using Domain.Entities.Identity;
using Application.Services.Licensing;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Identity.Roles.Commands;

[RequirePermission(ResourceType.Role, PermissionLevel.Write)]
public sealed record CreateRole(string Name, IEnumerable<PatchPermissionModel>? Permissions) : ICommand<Result<RoleDetails>>, IAdministratorRequest
{
    internal sealed class Validator : AbstractValidator<CreateRole>
    {
        public Validator()
        {
            RuleFor(x => x.Name).ValidNameIdentifier();
            RuleFor(x => x.Permissions).NotNull();
            RuleForEach(x => x.Permissions).SetValidator(new PermissionValidator());
        }
    }

    internal sealed class PermissionValidator : AbstractValidator<PatchPermissionModel>
    {
        public PermissionValidator()
        {
            RuleFor(x => x.ResourceType).IsInEnum();
            RuleFor(x => x.PermissionLevel).IsInEnum();
            RuleForEach(x => x.SpecificPermissions).IsInEnum();
            RuleFor(x => x)
                .Must(p => PermissionMatrix.IsAllowed(p.ResourceType, p.PermissionLevel, p.SpecificPermissions))
                .WithMessage(p => $"Invalid permission: [{p.ResourceType}]-[{p.PermissionLevel}] with specifics [{string.Join(", ", p.SpecificPermissions)}] is not an allowed combination.");
        }
    }
}

internal sealed class CreateRoleHandler(
    IUnitOfWork unitOfWork,
    ILicenseEntitlementService licenseEntitlementService) : ICommandHandler<CreateRole, Result<RoleDetails>>
{
    public async ValueTask<Result<RoleDetails>> Handle(CreateRole command, CancellationToken cancellationToken)
    {
        var exists = await unitOfWork.Roles.ExistsByNameAsync(command.Name, null, cancellationToken);
        if (exists)
            return Result.Failure<RoleDetails>(new ConflictError("Name already exists"));

        var entitlement = await licenseEntitlementService.EnsureEnabledAsync(
            LicenseCapability.CustomAccessControl,
            cancellationToken);
        if (entitlement.IsFailure())
            return Result.Failure<RoleDetails>(entitlement.Errors);

        var permissions = command.Permissions!.Select(x => x.ToDomain(Guid.Empty)).ToArray();
        var role = Role.Create(command.Name, RoleType.Custom, permissions);
        await unitOfWork.Roles.AddAsync(role, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        return new RoleDetails(role.Id, role.Name, role.RoleType, permissions);
    }
}
