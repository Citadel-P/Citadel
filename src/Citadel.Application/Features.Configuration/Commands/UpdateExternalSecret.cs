using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Configuration;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Configuration.Commands;

[RequirePermission(ResourceType.Configuration, PermissionLevel.Write)]
public sealed record UpdateExternalSecret(
    Guid Id,
    string Name,
    Guid ProviderId,
    string ExternalPath,
    string ExternalKey,
    int? ExternalVersion) : ICommand<Result<SecretDefinition>>
{
    internal sealed class Validator : AbstractValidator<UpdateExternalSecret>
    {
        public Validator()
        {
            RuleFor(x => x.Id).NotEmpty();
            RuleFor(x => x.Name)
                .NotEmpty()
                .MaximumLength(128)
                .Matches("^[A-Za-z_][A-Za-z0-9_]*$")
                .WithMessage("Secret name must be a valid environment variable name.");
            RuleFor(x => x.ProviderId).NotEmpty();
            RuleFor(x => x.ExternalPath).NotEmpty().MaximumLength(512);
            RuleFor(x => x.ExternalKey).NotEmpty().MaximumLength(256);
            RuleFor(x => x.ExternalVersion).GreaterThan(0).When(x => x.ExternalVersion.HasValue);
        }
    }
}

internal sealed class UpdateExternalSecretHandler(IUnitOfWork unitOfWork)
    : ICommandHandler<UpdateExternalSecret, Result<SecretDefinition>>
{
    public async ValueTask<Result<SecretDefinition>> Handle(UpdateExternalSecret command, CancellationToken cancellationToken)
    {
        var existing = await unitOfWork.SecretDefinitions.GetAsync(command.Id, cancellationToken);
        if (existing is null)
            return Result.Failure<SecretDefinition>(new NotFoundError("Secret definition not found."));

        if (existing.ProviderType != SecretProviderType.VaultCompatibleKvV2)
            return Result.Failure<SecretDefinition>(new BadRequestError("Only external Vault-compatible secrets can be updated with this operation."));

        if (await unitOfWork.SecretDefinitions.ExistsByNameExceptAsync(command.Name, command.Id, cancellationToken))
            return Result.Failure<SecretDefinition>(new ConflictError("Name already exists"));

        var provider = await unitOfWork.SecretProviders.GetAsync(command.ProviderId, cancellationToken);
        if (provider is null)
            return Result.Failure<SecretDefinition>(new NotFoundError("The provided secret provider does not exist."));
        if (provider.ProviderType != SecretProviderType.VaultCompatibleKvV2)
            return Result.Failure<SecretDefinition>(new BadRequestError("The provided secret provider is not supported for external secrets."));

        var secret = existing with
        {
            Name = command.Name,
            ProviderId = command.ProviderId,
            ExternalPath = command.ExternalPath.Trim('/'),
            ExternalKey = command.ExternalKey,
            ExternalVersion = command.ExternalVersion
        };

        try
        {
            secret.Validate();
        }
        catch (ArgumentException ex)
        {
            return Result.Failure<SecretDefinition>(new BadRequestError(ex.Message));
        }

        await unitOfWork.SecretDefinitions.UpdateAsync(secret, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        return secret;
    }
}
