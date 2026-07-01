using Domain.Contracts.Interfaces;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Configuration.Commands;

[RequirePermission(ResourceType.Binding, PermissionLevel.Write)]
public sealed record DeleteSecretProvider(Guid Id) : ICommand<Result>;

internal sealed class DeleteSecretProviderHandler(IUnitOfWork unitOfWork)
    : ICommandHandler<DeleteSecretProvider, Result>
{
    public async ValueTask<Result> Handle(DeleteSecretProvider command, CancellationToken cancellationToken)
    {
        var provider = await unitOfWork.SecretProviders.GetAsync(command.Id, cancellationToken);
        if (provider is null)
            return Result.Failure(new NotFoundError("Secret provider not found."));

        if (await unitOfWork.SecretProviders.IsUsedBySecretDefinitionAsync(command.Id, cancellationToken))
            return Result.Failure(new ConflictError("Secret provider is used by one or more external secrets."));

        await unitOfWork.SecretProviders.DeleteAsync(command.Id, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        return Result.Success();
    }
}
