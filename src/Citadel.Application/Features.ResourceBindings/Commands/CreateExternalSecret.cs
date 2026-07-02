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
public sealed record CreateExternalSecret(
    string Name,
    Guid ProviderId,
    string ExternalPath,
    string ExternalKey,
    int? ExternalVersion) : ICommand<Result<SecretDefinition>>
{
    internal sealed class Validator : AbstractValidator<CreateExternalSecret>
    {
        public Validator()
        {
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

internal sealed class CreateExternalSecretHandler(IUnitOfWork unitOfWork)
    : ICommandHandler<CreateExternalSecret, Result<SecretDefinition>>
{
    public async ValueTask<Result<SecretDefinition>> Handle(CreateExternalSecret command, CancellationToken cancellationToken)
    {
        if (await unitOfWork.SecretDefinitions.ExistsByNameAsync(command.Name, cancellationToken))
            return Result.Failure<SecretDefinition>(new ConflictError("Name already exists"));

        var provider = await unitOfWork.SecretProviders.GetAsync(command.ProviderId, cancellationToken);
        if (provider is null)
            return Result.Failure<SecretDefinition>(new NotFoundError("The provided secret provider does not exist."));
        if (provider.ProviderType != SecretProviderType.VaultCompatibleKvV2)
            return Result.Failure<SecretDefinition>(new BadRequestError("The provided secret provider is not supported for external secrets."));

        var secret = new SecretDefinition(
            command.Name,
            SecretProviderType.VaultCompatibleKvV2,
            command.ProviderId,
            command.ExternalPath.Trim('/'),
            command.ExternalKey,
            command.ExternalVersion);

        try
        {
            secret.Validate();
        }
        catch (ArgumentException ex)
        {
            return Result.Failure<SecretDefinition>(new BadRequestError(ex.Message));
        }

        await unitOfWork.SecretDefinitions.AddAsync(secret, value: null, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        return secret;
    }
}
