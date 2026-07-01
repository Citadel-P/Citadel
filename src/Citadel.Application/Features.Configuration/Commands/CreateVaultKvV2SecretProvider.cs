using Application.Services;
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
public sealed record CreateVaultKvV2SecretProvider(
    string Name,
    string Address,
    string MountPath,
    string Token) : ICommand<Result<SecretProvider>>
{
    internal sealed class Validator : AbstractValidator<CreateVaultKvV2SecretProvider>
    {
        public Validator()
        {
            RuleFor(x => x.Name).NotEmpty().MaximumLength(128);
            RuleFor(x => x.Address).NotEmpty().MaximumLength(512);
            RuleFor(x => x.MountPath).NotEmpty().MaximumLength(128);
            RuleFor(x => x.Token).NotNull();
        }
    }
}

internal sealed class CreateVaultKvV2SecretProviderHandler(
    IUnitOfWork unitOfWork,
    ISecretValueProtector secretValueProtector)
    : ICommandHandler<CreateVaultKvV2SecretProvider, Result<SecretProvider>>
{
    public async ValueTask<Result<SecretProvider>> Handle(CreateVaultKvV2SecretProvider command, CancellationToken cancellationToken)
    {
        if (await unitOfWork.SecretProviders.ExistsByNameAsync(command.Name, cancellationToken))
            return Result.Failure<SecretProvider>(new ConflictError("Name already exists"));

        var provider = new SecretProvider(
            command.Name,
            SecretProviderType.VaultCompatibleKvV2,
            new VaultKvV2SecretProviderConfiguration(
                command.Address.TrimEnd('/'),
                command.MountPath.Trim('/'),
                secretValueProtector.Protect(command.Token)));

        try
        {
            provider.Validate();
        }
        catch (ArgumentException ex)
        {
            return Result.Failure<SecretProvider>(new BadRequestError(ex.Message));
        }

        await unitOfWork.SecretProviders.AddAsync(provider, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        return provider;
    }
}
