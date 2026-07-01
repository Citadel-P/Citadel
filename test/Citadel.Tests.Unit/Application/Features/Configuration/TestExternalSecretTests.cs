using Application.Features.Configuration.Commands;
using Application.Services;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.ResourceBindings;
using Moq;

namespace Tests.Unit.Application.Features.Configuration;

public sealed class TestExternalSecretTests
{
    [Fact]
    public async Task TestVaultProviderConnection_Should_Use_Stored_Token_When_Command_Token_Is_Blank()
    {
        var providerId = Guid.CreateVersion7();
        var provider = CreateProvider(providerId);
        var unitOfWork = CreateUnitOfWork(provider);
        var externalClient = new Mock<IExternalSecretProviderClient>();
        externalClient
            .Setup(x => x.TestConnectionAsync(
                It.Is<SecretProvider>(p =>
                    p.Id == providerId &&
                    p.Name == "vault-edit" &&
                    p.Configuration.Address == "https://vault.local" &&
                    p.Configuration.MountPath == "kv"),
                "plain-token",
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(ExternalSecretProviderConnectionTestResult.Succeeded("ok"));
        var handler = new TestVaultKvV2SecretProviderConnectionHandler(
            unitOfWork.Object,
            new TestSecretValueProtector(),
            externalClient.Object);

        var result = await handler.Handle(
            new TestVaultKvV2SecretProviderConnection(providerId, "vault-edit", "https://vault.local/", "/kv/", ""),
            TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess(out var testResult, out var error), error?.Message);
        Assert.True(testResult.Success);
        Assert.Equal(
            "Connection successful using the stored token. Vault is reachable and the token is valid.",
            testResult.Message);
        unitOfWork.Verify(x => x.CommitAsync(It.IsAny<CancellationToken>()), Times.Never);
    }

    [Fact]
    public async Task TestVaultProviderConnection_Should_Return_Failure_When_Create_Token_Is_Blank()
    {
        var unitOfWork = new Mock<IUnitOfWork>();
        var externalClient = new Mock<IExternalSecretProviderClient>();
        var handler = new TestVaultKvV2SecretProviderConnectionHandler(
            unitOfWork.Object,
            new TestSecretValueProtector(),
            externalClient.Object);

        var result = await handler.Handle(
            new TestVaultKvV2SecretProviderConnection(null, "vault", "https://vault.local", "secret", ""),
            TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess(out var testResult, out var error), error?.Message);
        Assert.False(testResult.Success);
        Assert.Equal("Vault token is required to test the connection.", testResult.Message);
        externalClient.Verify(
            x => x.TestConnectionAsync(It.IsAny<SecretProvider>(), It.IsAny<string>(), It.IsAny<CancellationToken>()),
            Times.Never);
    }

    [Fact]
    public async Task TestExternalSecret_Should_Resolve_Reference_Without_Persisting_Secret_Value()
    {
        var providerId = Guid.CreateVersion7();
        var provider = CreateProvider(providerId);
        var unitOfWork = CreateUnitOfWork(provider);
        var externalClient = new Mock<IExternalSecretProviderClient>();
        externalClient
            .Setup(x => x.ResolveAsync(
                It.Is<SecretDefinition>(secret =>
                    secret.Name == "EXTERNAL_SECRET_TEST" &&
                    secret.ProviderId == providerId &&
                    secret.ExternalPath == "apps/api/prod" &&
                    secret.ExternalKey == "api_key" &&
                    secret.ExternalVersion == 3),
                provider,
                "plain-token",
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(ExternalSecretValueResult.Success("super-secret"));
        var handler = CreateHandler(unitOfWork.Object, externalClient.Object);

        var result = await handler.Handle(
            new TestExternalSecret(providerId, "/apps/api/prod/", "api_key", 3),
            TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess(out var testResult, out var error), error?.Message);
        Assert.True(testResult.Success);
        Assert.DoesNotContain("super-secret", testResult.Message, StringComparison.Ordinal);
        unitOfWork.Verify(x => x.SecretDefinitions, Times.Never);
        unitOfWork.Verify(x => x.CommitAsync(It.IsAny<CancellationToken>()), Times.Never);
    }

    [Fact]
    public async Task TestExternalSecret_Should_Return_Safe_Failure_When_External_Secret_Cannot_Be_Resolved()
    {
        var providerId = Guid.CreateVersion7();
        var provider = CreateProvider(providerId);
        var unitOfWork = CreateUnitOfWork(provider);
        var externalClient = new Mock<IExternalSecretProviderClient>();
        externalClient
            .Setup(x => x.ResolveAsync(
                It.IsAny<SecretDefinition>(),
                provider,
                "plain-token",
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(ExternalSecretValueResult.Failure("Vault returned HTTP 403."));
        var handler = CreateHandler(unitOfWork.Object, externalClient.Object);

        var result = await handler.Handle(
            new TestExternalSecret(providerId, "apps/api/prod", "api_key", null),
            TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess(out var testResult, out var error), error?.Message);
        Assert.False(testResult.Success);
        Assert.Equal("Vault returned HTTP 403.", testResult.Message);
        unitOfWork.Verify(x => x.CommitAsync(It.IsAny<CancellationToken>()), Times.Never);
    }

    private static SecretProvider CreateProvider(Guid providerId)
        => new(
            "vault",
            SecretProviderType.VaultCompatibleKvV2,
            new VaultKvV2SecretProviderConfiguration("https://vault.local", "secret", "protected-token"))
        {
            Id = providerId
        };

    private static Mock<IUnitOfWork> CreateUnitOfWork(SecretProvider provider)
    {
        var providers = new Mock<ISecretProviderRepository>();
        providers
            .Setup(x => x.GetAsync(provider.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync(provider);

        var unitOfWork = new Mock<IUnitOfWork>();
        unitOfWork.Setup(x => x.SecretProviders).Returns(providers.Object);
        return unitOfWork;
    }

    private static TestExternalSecretHandler CreateHandler(
        IUnitOfWork unitOfWork,
        IExternalSecretProviderClient externalClient)
        => new(
            unitOfWork,
            new TestSecretValueProtector(),
            externalClient);

    private sealed class TestSecretValueProtector : ISecretValueProtector
    {
        public string Protect(string value) => value;

        public string Unprotect(string protectedValue) => "plain-token";
    }
}
