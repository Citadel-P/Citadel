using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.ResourceBindings;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.ResourceBindings.Commands;

[RequirePermission(ResourceType.Binding, PermissionLevel.Write)]
public sealed record UpdateExternalSecret(
    Guid Id,
    string? Name,
    Guid? ProviderId,
    string? ExternalPath,
    string? ExternalKey,
    int? ExternalVersion,
    bool ExternalVersionSpecified = true) : ICommand<Result<SecretDefinition>>
{
    internal sealed class Validator : AbstractValidator<UpdateExternalSecret>
    {
        public Validator()
        {
            RuleFor(x => x.Id).NotEmpty();
            RuleFor(x => x.Name)
                .MaximumLength(128)
                .Matches("^[A-Za-z_][A-Za-z0-9_]*$")
                .WithMessage("Secret name must be a valid environment variable name.")
                .When(x => x.Name is not null);
            RuleFor(x => x.ProviderId)
                .NotEmpty()
                .When(x => x.ProviderId.HasValue);
            RuleFor(x => x.ExternalPath)
                .NotEmpty()
                .MaximumLength(512)
                .When(x => x.ExternalPath is not null);
            RuleFor(x => x.ExternalKey)
                .NotEmpty()
                .MaximumLength(256)
                .When(x => x.ExternalKey is not null);
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

        var name = command.Name ?? existing.Name;
        var providerId = command.ProviderId ?? existing.ProviderId;
        var externalPath = command.ExternalPath ?? existing.ExternalPath;
        var externalKey = command.ExternalKey ?? existing.ExternalKey;
        var externalVersion = command.ExternalVersionSpecified ? command.ExternalVersion : existing.ExternalVersion;

        if (providerId is null)
            return Result.Failure<SecretDefinition>(new BadRequestError("External secrets require a provider."));

        if (await unitOfWork.SecretDefinitions.ExistsByNameExceptAsync(name, command.Id, cancellationToken))
            return Result.Failure<SecretDefinition>(new ConflictError("Name already exists"));

        var provider = await unitOfWork.SecretProviders.GetAsync(providerId.Value, cancellationToken);
        if (provider is null)
            return Result.Failure<SecretDefinition>(new NotFoundError("The provided secret provider does not exist."));
        if (provider.ProviderType != SecretProviderType.VaultCompatibleKvV2)
            return Result.Failure<SecretDefinition>(new BadRequestError("The provided secret provider is not supported for external secrets."));

        var secret = existing with
        {
            Name = name,
            ProviderId = providerId,
            ExternalPath = externalPath?.Trim('/'),
            ExternalKey = externalKey,
            ExternalVersion = externalVersion
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
