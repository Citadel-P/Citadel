using Application.Services;
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
public sealed record UpdateVaultKvV2SecretProvider(
    Guid Id,
    string? Name,
    string? Address,
    string? MountPath,
    string? Token) : ICommand<Result<SecretProvider>>
{
    internal sealed class Validator : AbstractValidator<UpdateVaultKvV2SecretProvider>
    {
        public Validator()
        {
            RuleFor(x => x.Id).NotEmpty();
            RuleFor(x => x.Name)
                .NotEmpty()
                .MaximumLength(128)
                .When(x => x.Name is not null);
            RuleFor(x => x.Address)
                .NotEmpty()
                .MaximumLength(512)
                .When(x => x.Address is not null);
            RuleFor(x => x.MountPath)
                .NotEmpty()
                .MaximumLength(128)
                .When(x => x.MountPath is not null);
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

        var name = command.Name ?? existing.Name;
        var address = command.Address ?? existing.Configuration.Address;
        var mountPath = command.MountPath ?? existing.Configuration.MountPath;

        if (await unitOfWork.SecretProviders.ExistsByNameExceptAsync(name, command.Id, cancellationToken))
            return Result.Failure<SecretProvider>(new ConflictError("Name already exists"));

        var protectedToken = string.IsNullOrWhiteSpace(command.Token)
            ? existing.Configuration.ProtectedToken
            : secretValueProtector.Protect(command.Token);

        var provider = existing with
        {
            Name = name,
            Configuration = new VaultKvV2SecretProviderConfiguration(
                address.TrimEnd('/'),
                mountPath.Trim('/'),
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
