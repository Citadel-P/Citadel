using Application.Features.ResourceBindings.Commands;
using Application.Services;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.ResourceBindings;
using Moq;

namespace Tests.Unit.Application.Features.ResourceBindings;

public sealed class CreateSecretTests
{
    [Fact]
    public async Task CreateInternalSecret_Should_Reject_Duplicate_Name()
    {
        var secrets = new Mock<ISecretDefinitionRepository>();
        secrets
            .Setup(x => x.ExistsByNameAsync("API_KEY", It.IsAny<CancellationToken>()))
            .ReturnsAsync(true);
        var unitOfWork = CreateUnitOfWork(secrets.Object);
        var handler = new CreateInternalSecretHandler(unitOfWork.Object, new TestSecretValueProtector());

        var result = await handler.Handle(new CreateInternalSecret("API_KEY", "secret"), TestContext.Current.CancellationToken);

        Assert.False(result.IsSuccess(out _, out var error));
        Assert.Equal("Name already exists", error.Message);
        secrets.Verify(x => x.AddAsync(It.IsAny<SecretDefinition>(), It.IsAny<InternalSecretValue>(), It.IsAny<CancellationToken>()), Times.Never);
        unitOfWork.Verify(x => x.CommitAsync(It.IsAny<CancellationToken>()), Times.Never);
    }

    [Fact]
    public async Task CreateExternalSecret_Should_Reject_Duplicate_Name_Before_Provider_Lookup()
    {
        var secrets = new Mock<ISecretDefinitionRepository>();
        secrets
            .Setup(x => x.ExistsByNameAsync("API_KEY", It.IsAny<CancellationToken>()))
            .ReturnsAsync(true);
        var providers = new Mock<ISecretProviderRepository>();
        var unitOfWork = CreateUnitOfWork(secrets.Object, providers.Object);
        var handler = new CreateExternalSecretHandler(unitOfWork.Object);

        var result = await handler.Handle(
            new CreateExternalSecret("API_KEY", Guid.CreateVersion7(), "apps/api/prod", "api_key", null),
            TestContext.Current.CancellationToken);

        Assert.False(result.IsSuccess(out _, out var error));
        Assert.Equal("Name already exists", error.Message);
        providers.Verify(x => x.GetAsync(It.IsAny<Guid>(), It.IsAny<CancellationToken>()), Times.Never);
        secrets.Verify(x => x.AddAsync(It.IsAny<SecretDefinition>(), null, It.IsAny<CancellationToken>()), Times.Never);
        unitOfWork.Verify(x => x.CommitAsync(It.IsAny<CancellationToken>()), Times.Never);
    }

    [Fact]
    public async Task UpdateVaultProvider_Should_Preserve_Token_When_Token_Is_Blank()
    {
        var providerId = Guid.CreateVersion7();
        var existing = new SecretProvider(
            "vault",
            SecretProviderType.VaultCompatibleKvV2,
            new VaultKvV2SecretProviderConfiguration("https://vault.local", "secret", "protected:old"))
        {
            Id = providerId
        };
        var providers = new Mock<ISecretProviderRepository>();
        providers
            .Setup(x => x.GetAsync(providerId, It.IsAny<CancellationToken>()))
            .ReturnsAsync(existing);
        var unitOfWork = CreateUnitOfWork(new Mock<ISecretDefinitionRepository>().Object, providers.Object);
        var handler = new UpdateVaultKvV2SecretProviderHandler(unitOfWork.Object, new TestSecretValueProtector());

        var result = await handler.Handle(
            new UpdateVaultKvV2SecretProvider(providerId, "vault-main", "https://vault.local/", "/kv/", ""),
            TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess(out var provider, out _));
        Assert.Equal("vault-main", provider.Name);
        Assert.Equal("https://vault.local", provider.Configuration.Address);
        Assert.Equal("kv", provider.Configuration.MountPath);
        Assert.Equal("protected:old", provider.Configuration.ProtectedToken);
        providers.Verify(
            x => x.UpdateAsync(
                It.Is<SecretProvider>(p =>
                    p.Id == providerId &&
                    p.Name == "vault-main" &&
                    p.Configuration.ProtectedToken == "protected:old"),
                It.IsAny<CancellationToken>()),
            Times.Once);
        unitOfWork.Verify(x => x.CommitAsync(It.IsAny<CancellationToken>()), Times.Once);
    }

    [Fact]
    public async Task DeleteSecretProvider_Should_Reject_Provider_Used_By_External_Secrets()
    {
        var providerId = Guid.CreateVersion7();
        var provider = new SecretProvider(
            "vault",
            SecretProviderType.VaultCompatibleKvV2,
            new VaultKvV2SecretProviderConfiguration("https://vault.local", "secret", "protected:old"))
        {
            Id = providerId
        };
        var providers = new Mock<ISecretProviderRepository>();
        providers
            .Setup(x => x.GetAsync(providerId, It.IsAny<CancellationToken>()))
            .ReturnsAsync(provider);
        providers
            .Setup(x => x.IsUsedByResourceBindingAsync(providerId, It.IsAny<CancellationToken>()))
            .ReturnsAsync(true);
        var unitOfWork = CreateUnitOfWork(new Mock<ISecretDefinitionRepository>().Object, providers.Object);
        var handler = new DeleteSecretProviderHandler(unitOfWork.Object);

        var result = await handler.Handle(new DeleteSecretProvider(providerId), TestContext.Current.CancellationToken);

        Assert.True(result.IsFailure(out var error));
        Assert.Equal("Secret provider is used by one or more external secrets.", error.Message);
        providers.Verify(x => x.DeleteAsync(It.IsAny<Guid>(), It.IsAny<CancellationToken>()), Times.Never);
        unitOfWork.Verify(x => x.CommitAsync(It.IsAny<CancellationToken>()), Times.Never);
    }

    [Fact]
    public async Task DeleteSecretProvider_Should_Delete_Unused_External_Secrets()
    {
        var providerId = Guid.CreateVersion7();
        var provider = new SecretProvider(
            "vault",
            SecretProviderType.VaultCompatibleKvV2,
            new VaultKvV2SecretProviderConfiguration("https://vault.local", "secret", "protected:old"))
        {
            Id = providerId
        };
        var secrets = new Mock<ISecretDefinitionRepository>();
        var providers = new Mock<ISecretProviderRepository>();
        providers
            .Setup(x => x.GetAsync(providerId, It.IsAny<CancellationToken>()))
            .ReturnsAsync(provider);
        providers
            .Setup(x => x.IsUsedByResourceBindingAsync(providerId, It.IsAny<CancellationToken>()))
            .ReturnsAsync(false);
        var unitOfWork = CreateUnitOfWork(secrets.Object, providers.Object);
        var handler = new DeleteSecretProviderHandler(unitOfWork.Object);

        var result = await handler.Handle(new DeleteSecretProvider(providerId), TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess());
        secrets.Verify(x => x.DeleteExternalByProviderIdAsync(providerId, It.IsAny<CancellationToken>()), Times.Once);
        providers.Verify(x => x.DeleteAsync(providerId, It.IsAny<CancellationToken>()), Times.Once);
        unitOfWork.Verify(x => x.CommitAsync(It.IsAny<CancellationToken>()), Times.Once);
    }

    [Fact]
    public async Task DeleteSecretDefinition_Should_Reject_Secret_Used_By_Binding()
    {
        var secretId = Guid.CreateVersion7();
        var secret = new SecretDefinition("API_KEY", SecretProviderType.InternalEncrypted) { Id = secretId };
        var secrets = new Mock<ISecretDefinitionRepository>();
        secrets.Setup(x => x.GetAsync(secretId, It.IsAny<CancellationToken>())).ReturnsAsync(secret);
        secrets.Setup(x => x.IsUsedByResourceBindingAsync(secretId, It.IsAny<CancellationToken>())).ReturnsAsync(true);
        var unitOfWork = CreateUnitOfWork(secrets.Object);
        var handler = new DeleteSecretDefinitionHandler(unitOfWork.Object);

        var result = await handler.Handle(new DeleteSecretDefinition(secretId), TestContext.Current.CancellationToken);

        Assert.True(result.IsFailure(out var error));
        Assert.Equal("Secret is used by one or more resource bindings.", error.Message);
        secrets.Verify(x => x.DeleteAsync(It.IsAny<Guid>(), It.IsAny<CancellationToken>()), Times.Never);
        unitOfWork.Verify(x => x.CommitAsync(It.IsAny<CancellationToken>()), Times.Never);
    }

    [Fact]
    public async Task DeleteSecretDefinition_Should_Delete_Unused_Secret()
    {
        var secretId = Guid.CreateVersion7();
        var secret = new SecretDefinition("API_KEY", SecretProviderType.InternalEncrypted) { Id = secretId };
        var secrets = new Mock<ISecretDefinitionRepository>();
        secrets.Setup(x => x.GetAsync(secretId, It.IsAny<CancellationToken>())).ReturnsAsync(secret);
        secrets.Setup(x => x.IsUsedByResourceBindingAsync(secretId, It.IsAny<CancellationToken>())).ReturnsAsync(false);
        var unitOfWork = CreateUnitOfWork(secrets.Object);
        var handler = new DeleteSecretDefinitionHandler(unitOfWork.Object);

        var result = await handler.Handle(new DeleteSecretDefinition(secretId), TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess());
        secrets.Verify(x => x.DeleteAsync(secretId, It.IsAny<CancellationToken>()), Times.Once);
        unitOfWork.Verify(x => x.CommitAsync(It.IsAny<CancellationToken>()), Times.Once);
    }

    [Fact]
    public async Task UpdateExternalSecret_Should_Update_Vault_Metadata()
    {
        var secretId = Guid.CreateVersion7();
        var providerId = Guid.CreateVersion7();
        var existing = new SecretDefinition(
            "STRIPE_API_KEY",
            SecretProviderType.VaultCompatibleKvV2,
            providerId,
            "apps/api/prod",
            "stripe_api_key",
            null)
        {
            Id = secretId
        };
        var provider = new SecretProvider(
            "vault",
            SecretProviderType.VaultCompatibleKvV2,
            new VaultKvV2SecretProviderConfiguration("https://vault.local", "secret", "protected"))
        {
            Id = providerId
        };
        var secrets = new Mock<ISecretDefinitionRepository>();
        secrets.Setup(x => x.GetAsync(secretId, It.IsAny<CancellationToken>())).ReturnsAsync(existing);
        secrets.Setup(x => x.ExistsByNameExceptAsync("STRIPE_SECRET", secretId, It.IsAny<CancellationToken>())).ReturnsAsync(false);
        var providers = new Mock<ISecretProviderRepository>();
        providers.Setup(x => x.GetAsync(providerId, It.IsAny<CancellationToken>())).ReturnsAsync(provider);
        var unitOfWork = CreateUnitOfWork(secrets.Object, providers.Object);
        var handler = new UpdateExternalSecretHandler(unitOfWork.Object);

        var result = await handler.Handle(
            new UpdateExternalSecret(secretId, "STRIPE_SECRET", providerId, "/apps/api/staging/", "stripe_secret", 2),
            TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess(out var secret, out var error), error?.Message);
        Assert.Equal(secretId, secret.Id);
        Assert.Equal("STRIPE_SECRET", secret.Name);
        Assert.Equal("apps/api/staging", secret.ExternalPath);
        Assert.Equal("stripe_secret", secret.ExternalKey);
        Assert.Equal(2, secret.ExternalVersion);
        secrets.Verify(
            x => x.UpdateAsync(
                It.Is<SecretDefinition>(s =>
                    s.Id == secretId &&
                    s.Name == "STRIPE_SECRET" &&
                    s.ExternalPath == "apps/api/staging" &&
                    s.ExternalKey == "stripe_secret" &&
                    s.ExternalVersion == 2),
                It.IsAny<CancellationToken>()),
            Times.Once);
        unitOfWork.Verify(x => x.CommitAsync(It.IsAny<CancellationToken>()), Times.Once);
    }

    [Fact]
    public async Task UpdateExternalSecret_Should_Reject_Duplicate_Name()
    {
        var secretId = Guid.CreateVersion7();
        var providerId = Guid.CreateVersion7();
        var existing = new SecretDefinition(
            "STRIPE_API_KEY",
            SecretProviderType.VaultCompatibleKvV2,
            providerId,
            "apps/api/prod",
            "stripe_api_key",
            null)
        {
            Id = secretId
        };
        var secrets = new Mock<ISecretDefinitionRepository>();
        secrets.Setup(x => x.GetAsync(secretId, It.IsAny<CancellationToken>())).ReturnsAsync(existing);
        secrets.Setup(x => x.ExistsByNameExceptAsync("EXISTING", secretId, It.IsAny<CancellationToken>())).ReturnsAsync(true);
        var providers = new Mock<ISecretProviderRepository>();
        var unitOfWork = CreateUnitOfWork(secrets.Object, providers.Object);
        var handler = new UpdateExternalSecretHandler(unitOfWork.Object);

        var result = await handler.Handle(
            new UpdateExternalSecret(secretId, "EXISTING", providerId, "apps/api/prod", "stripe_api_key", null),
            TestContext.Current.CancellationToken);

        Assert.False(result.IsSuccess(out _, out var error));
        Assert.Equal("Name already exists", error.Message);
        providers.Verify(x => x.GetAsync(It.IsAny<Guid>(), It.IsAny<CancellationToken>()), Times.Never);
        secrets.Verify(x => x.UpdateAsync(It.IsAny<SecretDefinition>(), It.IsAny<CancellationToken>()), Times.Never);
        unitOfWork.Verify(x => x.CommitAsync(It.IsAny<CancellationToken>()), Times.Never);
    }

    private static Mock<IUnitOfWork> CreateUnitOfWork(
        ISecretDefinitionRepository secrets,
        ISecretProviderRepository? providers = null)
    {
        var unitOfWork = new Mock<IUnitOfWork>();
        unitOfWork.Setup(x => x.SecretDefinitions).Returns(secrets);
        if (providers is not null)
            unitOfWork.Setup(x => x.SecretProviders).Returns(providers);
        return unitOfWork;
    }

    private sealed class TestSecretValueProtector : ISecretValueProtector
    {
        public string Protect(string value) => $"protected:{value}";

        public string Unprotect(string protectedValue) => protectedValue;
    }
}
