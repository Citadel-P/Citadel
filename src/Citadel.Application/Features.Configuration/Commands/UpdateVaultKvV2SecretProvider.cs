using Application.Services;
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
public sealed record UpdateVaultKvV2SecretProvider(
    Guid Id,
    string Name,
    string Address,
    string MountPath,
    string? Token) : ICommand<Result<SecretProvider>>
{
    internal sealed class Validator : AbstractValidator<UpdateVaultKvV2SecretProvider>
    {
        public Validator()
        {
            RuleFor(x => x.Id).NotEmpty();
            RuleFor(x => x.Name).NotEmpty().MaximumLength(128);
            RuleFor(x => x.Address).NotEmpty().MaximumLength(512);
            RuleFor(x => x.MountPath).NotEmpty().MaximumLength(128);
        }
    }
}

internal sealed class UpdateVaultKvV2SecretProviderHandler(
    IUnitOfWork unitOfWork,
    ISecretValueProtector secretValueProtector)
    : ICommandHandler<UpdateVaultKvV2SecretProvider, Result<SecretProvider>>
{
    public async ValueTask<Result<SecretProvider>> Handle(UpdateVaultKvV2SecretProvider command, CancellationToken cancellationToken)
    {
        var existing = await unitOfWork.SecretProviders.GetAsync(command.Id, cancellationToken);
        if (existing is null)
            return Result.Failure<SecretProvider>(new NotFoundError("Secret provider not found."));

        if (await unitOfWork.SecretProviders.ExistsByNameExceptAsync(command.Name, command.Id, cancellationToken))
            return Result.Failure<SecretProvider>(new ConflictError("Name already exists"));

        var protectedToken = string.IsNullOrWhiteSpace(command.Token)
            ? existing.Configuration.ProtectedToken
            : secretValueProtector.Protect(command.Token);

        var provider = existing with
        {
            Name = command.Name,
            Configuration = new VaultKvV2SecretProviderConfiguration(
                command.Address.TrimEnd('/'),
                command.MountPath.Trim('/'),
                protectedToken),
            UpdatedAt = DateTime.UtcNow
        };

        try
        {
            provider.Validate();
        }
        catch (ArgumentException ex)
        {
            return Result.Failure<SecretProvider>(new BadRequestError(ex.Message));
        }

        await unitOfWork.SecretProviders.UpdateAsync(provider, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        return provider;
    }
}
