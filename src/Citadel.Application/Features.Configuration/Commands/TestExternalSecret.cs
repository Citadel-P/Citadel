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
public sealed record TestExternalSecret(
    Guid ProviderId,
    string ExternalPath,
    string ExternalKey,
    int? ExternalVersion) : ICommand<Result<ExternalSecretTestResult>>
{
    internal sealed class Validator : AbstractValidator<TestExternalSecret>
    {
        public Validator()
        {
            RuleFor(x => x.ProviderId).NotEmpty();
            RuleFor(x => x.ExternalPath).NotEmpty().MaximumLength(512);
            RuleFor(x => x.ExternalKey).NotEmpty().MaximumLength(256);
            RuleFor(x => x.ExternalVersion).GreaterThan(0).When(x => x.ExternalVersion.HasValue);
        }
    }
}

public sealed record ExternalSecretTestResult(bool Success, string Message);

internal sealed class TestExternalSecretHandler(
    IUnitOfWork unitOfWork,
    ISecretValueProtector secretValueProtector,
    IExternalSecretProviderClient externalSecretProviderClient)
    : ICommandHandler<TestExternalSecret, Result<ExternalSecretTestResult>>
{
    public async ValueTask<Result<ExternalSecretTestResult>> Handle(TestExternalSecret command, CancellationToken cancellationToken)
    {
        var provider = await unitOfWork.SecretProviders.GetAsync(command.ProviderId, cancellationToken);
        if (provider is null)
            return Result.Failure<ExternalSecretTestResult>(new NotFoundError("The provided secret provider does not exist."));

        if (provider.ProviderType != SecretProviderType.VaultCompatibleKvV2)
            return Result.Failure<ExternalSecretTestResult>(new BadRequestError("The provided secret provider is not supported for external secrets."));

        var secret = new SecretDefinition(
            "EXTERNAL_SECRET_TEST",
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
            return Result.Failure<ExternalSecretTestResult>(new BadRequestError(ex.Message));
        }

        string token;
        try
        {
            token = secretValueProtector.Unprotect(provider.Configuration.ProtectedToken);
        }
        catch (Exception ex) when (ex is FormatException or CryptographicException)
        {
            return Result.Success(new ExternalSecretTestResult(false, "Secret provider token could not be decrypted."));
        }

        var result = await externalSecretProviderClient.ResolveAsync(secret, provider, token, cancellationToken);
        return result.IsSuccess
            ? Result.Success(new ExternalSecretTestResult(true, "External secret reference resolved successfully."))
            : Result.Success(new ExternalSecretTestResult(false, result.ErrorMessage ?? "External secret reference could not be resolved."));
    }
}
