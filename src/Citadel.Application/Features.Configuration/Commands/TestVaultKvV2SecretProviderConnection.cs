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
using System.Security.Cryptography;

namespace Application.Features.Configuration.Commands;

[RequirePermission(ResourceType.Configuration, PermissionLevel.Write)]
public sealed record TestVaultKvV2SecretProviderConnection(
    Guid? ProviderId,
    string? Name,
    string Address,
    string MountPath,
    string? Token) : ICommand<Result<SecretProviderConnectionTestResult>>
{
    internal sealed class Validator : AbstractValidator<TestVaultKvV2SecretProviderConnection>
    {
        public Validator()
        {
            RuleFor(x => x.Address).NotEmpty().MaximumLength(512);
            RuleFor(x => x.MountPath).NotEmpty().MaximumLength(128);
        }
    }
}

public sealed record SecretProviderConnectionTestResult(bool Success, string Message);

internal sealed class TestVaultKvV2SecretProviderConnectionHandler(
    IUnitOfWork unitOfWork,
    ISecretValueProtector secretValueProtector,
    IExternalSecretProviderClient externalSecretProviderClient)
    : ICommandHandler<TestVaultKvV2SecretProviderConnection, Result<SecretProviderConnectionTestResult>>
{
    public async ValueTask<Result<SecretProviderConnectionTestResult>> Handle(
        TestVaultKvV2SecretProviderConnection command,
        CancellationToken cancellationToken)
    {
        SecretProvider? existing = null;
        if (command.ProviderId is { } providerId)
        {
            existing = await unitOfWork.SecretProviders.GetAsync(providerId, cancellationToken);
            if (existing is null)
                return Result.Failure<SecretProviderConnectionTestResult>(new NotFoundError("Secret provider not found."));
        }

        var token = command.Token;
        var usedStoredToken = false;
        if (string.IsNullOrWhiteSpace(token) && existing is not null)
        {
            try
            {
                token = secretValueProtector.Unprotect(existing.Configuration.ProtectedToken);
                usedStoredToken = true;
            }
            catch (Exception ex) when (ex is FormatException or CryptographicException)
            {
                return Result.Success(new SecretProviderConnectionTestResult(false, "Secret provider token could not be decrypted."));
            }
        }

        if (string.IsNullOrWhiteSpace(token))
            return Result.Success(new SecretProviderConnectionTestResult(false, "Vault token is required to test the connection."));

        var provider = new SecretProvider(
            string.IsNullOrWhiteSpace(command.Name) ? (existing?.Name ?? "Vault provider") : command.Name,
            SecretProviderType.VaultCompatibleKvV2,
            new VaultKvV2SecretProviderConfiguration(
                command.Address.TrimEnd('/'),
                command.MountPath.Trim('/'),
                "test-token"))
        {
            Id = existing?.Id ?? Guid.CreateVersion7()
        };

        try
        {
            provider.Validate();
        }
        catch (ArgumentException ex)
        {
            return Result.Failure<SecretProviderConnectionTestResult>(new BadRequestError(ex.Message));
        }

        var result = await externalSecretProviderClient.TestConnectionAsync(provider, token, cancellationToken);
        if (usedStoredToken && result.Success)
        {
            return Result.Success(new SecretProviderConnectionTestResult(
                true,
                "Connection successful using the stored token. Vault is reachable and the token is valid."));
        }

        return Result.Success(new SecretProviderConnectionTestResult(result.Success, result.Message));
    }
}
