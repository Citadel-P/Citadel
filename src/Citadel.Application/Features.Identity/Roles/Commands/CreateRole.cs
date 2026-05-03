using Application.Features.Identity.Roles;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Role;
using Domain.Entities.Identity;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Identity.Roles.Commands;

[RequirePermission(ResourceType.Role, ResourceAction.Create)]
public sealed record CreateRole(string Name, IEnumerable<Permission> Permissions) : ICommand<Result<RoleDetails>>
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

    internal sealed class PermissionValidator : AbstractValidator<Permission>
    {
        public PermissionValidator()
        {
            RuleFor(x => x.ResourceType).IsInEnum();
            RuleFor(x => x.ResourceAction).IsInEnum();
            RuleFor(x => x)
                .Must(p => PermissionMatrix.IsAllowed(p.ResourceType, p.ResourceAction))
                .WithMessage(p => $"Invalid permission: [{p.ResourceType}]-[{p.ResourceAction}] is not an allowed combination.");
        }
    }
}

internal sealed class CreateRoleHandler(IUnitOfWork unitOfWork) : ICommandHandler<CreateRole, Result<RoleDetails>>
{
    public async ValueTask<Result<RoleDetails>> Handle(CreateRole command, CancellationToken cancellationToken)
    {
        var exists = await unitOfWork.Roles.ExistsByNameAsync(command.Name, null, cancellationToken);
        if (exists)
            return Result.Failure<RoleDetails>(new ConflictError("Name already exists"));

        var role = Role.Create(command.Name, RoleType.Custom, command.Permissions);
        await unitOfWork.Roles.AddAsync(role, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        return new RoleDetails(role.Id, role.Name, role.RoleType, command.Permissions);
    }
}
