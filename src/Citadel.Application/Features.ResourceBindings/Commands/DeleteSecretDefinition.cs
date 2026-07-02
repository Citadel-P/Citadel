using Domain.Contracts.Interfaces;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.ResourceBindings.Commands;

[RequirePermission(ResourceType.Binding, PermissionLevel.Write)]
public sealed record DeleteSecretDefinition(Guid Id) : ICommand<Result>;

internal sealed class DeleteSecretDefinitionHandler(IUnitOfWork unitOfWork)
    : ICommandHandler<DeleteSecretDefinition, Result>
{
    public async ValueTask<Result> Handle(DeleteSecretDefinition command, CancellationToken cancellationToken)
    {
        var secret = await unitOfWork.SecretDefinitions.GetAsync(command.Id, cancellationToken);
        if (secret is null)
            return Result.Failure(new NotFoundError("Secret definition not found."));

        if (await unitOfWork.SecretDefinitions.IsUsedByResourceBindingAsync(command.Id, cancellationToken))
            return Result.Failure(new ConflictError("Secret is used by one or more resource bindings."));

        await unitOfWork.SecretDefinitions.DeleteAsync(command.Id, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        return Result.Success();
    }
}
