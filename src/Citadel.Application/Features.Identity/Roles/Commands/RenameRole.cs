using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Role;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Identity.Roles.Commands;

[RequirePermission(ResourceType.Role, PermissionLevel.Write)]
public sealed record RenameRole(Guid Id, string Name) : ICommand<Result<RoleDetails>>
{
    internal sealed class Validator : AbstractValidator<RenameRole>
    {
        public Validator()
        {
            RuleFor(x => x.Id).NotEmpty();
            RuleFor(x => x.Name).NotEmpty().ValidNameIdentifier();
        }
    }
}

internal sealed class RenameRoleHandler(IUnitOfWork unitOfWork) : ICommandHandler<RenameRole, Result<RoleDetails>>
{
    public async ValueTask<Result<RoleDetails>> Handle(RenameRole command, CancellationToken cancellationToken)
    {
        var role = await unitOfWork.Roles.GetAsync(command.Id, cancellationToken);
        if (role is null)
            return Result.Failure<RoleDetails>(new NotFoundError("The provided role does not exist"));

        var exists = await unitOfWork.Roles.ExistsByNameAsync(command.Name, command.Id, cancellationToken);
        if (exists)
            return Result.Failure<RoleDetails>(new ConflictError("Name already exists"));

        var renameResult = role.Rename(command.Name);
        if (renameResult.IsFailure(out var error))
            return Result.Failure<RoleDetails>(error);

        await unitOfWork.Roles.RenameAsync(role, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        return new RoleDetails(role.Id, role.Name, role.RoleType, role.Permissions);
    }
}
