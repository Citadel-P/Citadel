using Application.Services;
using Domain;
using Domain.Contracts.Resources.Configuration;
using Domain.Contracts.Resources.Deployments;
using Domain.Entities.Activities;
using Domain.Contracts.Interfaces;
using Domain.Entities.Configuration;
using Microsoft.Extensions.DependencyInjection;
using Moq;
using System.Text.Json;

namespace Tests.Unit.Application.Services;

public class ConfigurationResolverTests
{
    [Fact]
    public async Task ResolveAsync_Should_Overlay_Resource_Entries_Over_Global_Entries()
    {
        var stackId = Guid.CreateVersion7();
        var entries = new[]
        {
            new ConfigurationEntry(
                Name: "APP_MODE",
                Kind: ConfigurationEntryKind.Variable,
                Scope: ConfigurationScope.Global,
                ResourceId: null,
                Value: "global",
                SecretId: null),
            new ConfigurationEntry(
                Name: "APP_MODE",
                Kind: ConfigurationEntryKind.Variable,
                Scope: ConfigurationScope.Stack,
                ResourceId: stackId,
                Value: "stack",
                SecretId: null),
            new ConfigurationEntry(
                Name: "SHARED",
                Kind: ConfigurationEntryKind.Variable,
                Scope: ConfigurationScope.Global,
                ResourceId: null,
                Value: "true",
                SecretId: null)
        };

        var resolver = CreateResolver(entries);

        var result = await resolver.ResolveAsync(ConfigurationScope.Stack, stackId, TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess(out var resolved, out var error), error?.Message);
        Assert.Equal(["APP_MODE=stack", "SHARED=true"], resolved.EnvironmentVariables);
        Assert.Empty(resolved.RedactionValues);
        Assert.Equal(2, resolved.VariableCount);
        Assert.Equal(0, resolved.SecretCount);
    }

    [Fact]
    public async Task ResolveAsync_Should_Resolve_Internal_Secrets_And_Return_Redaction_Values()
    {
        var deploymentId = Guid.CreateVersion7();
        var secretId = Guid.CreateVersion7();
        var entry = new ConfigurationEntry(
            Name: "API_KEY",
            Kind: ConfigurationEntryKind.Secret,
            Scope: ConfigurationScope.Deployment,
            ResourceId: deploymentId,
            Value: null,
            SecretId: secretId,
            SecretDeliveryMode: SecretDeliveryMode.EnvironmentVariable);
        var secret = new SecretDefinition(
            Name: "API_KEY",
            ProviderType: SecretProviderType.InternalEncrypted)
        {
            Id = secretId
        };
        var encryptedSecret = new InternalSecretValue(secretId, "encrypted");
        var resolver = CreateResolver(
            [entry],
            configureSecrets: secrets =>
            {
                secrets.Setup(x => x.GetAsync(secretId, It.IsAny<CancellationToken>())).ReturnsAsync(secret);
                secrets.Setup(x => x.GetInternalValueAsync(secretId, It.IsAny<CancellationToken>())).ReturnsAsync(encryptedSecret);
            },
            unprotectedSecretValue: "plain-secret");

        var result = await resolver.ResolveAsync(ConfigurationScope.Deployment, deploymentId, TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess(out var resolved, out var error), error?.Message);
        Assert.Equal(["API_KEY=plain-secret"], resolved.EnvironmentVariables);
        Assert.Equal(["plain-secret"], resolved.RedactionValues);
        Assert.Equal(0, resolved.VariableCount);
        Assert.Equal(1, resolved.SecretCount);
        var snapshot = Assert.Single(resolved.SnapshotEntries);
        Assert.Equal("API_KEY", snapshot.Name);
        Assert.Equal(ConfigurationEntryKind.Secret, snapshot.Kind);
        Assert.Equal(ConfigurationScope.Deployment, snapshot.Scope);
        Assert.Equal(deploymentId, snapshot.ResourceId);
        Assert.Equal("********", snapshot.Value);
        Assert.Equal(secretId, snapshot.SecretId);
        Assert.Equal("API_KEY", snapshot.SecretName);
        Assert.Equal(SecretProviderType.InternalEncrypted, snapshot.SecretProviderType);
        Assert.Null(snapshot.SecretProviderName);
        Assert.Equal(SecretDeliveryMode.EnvironmentVariable, snapshot.SecretDeliveryMode);
    }

    [Fact]
    public async Task ResolveAsync_Should_Resolve_External_Secrets_And_Return_Redaction_Values()
    {
        var stackId = Guid.CreateVersion7();
        var secretId = Guid.CreateVersion7();
        var providerId = Guid.CreateVersion7();
        var entry = new ConfigurationEntry(
            Name: "API_TOKEN",
            Kind: ConfigurationEntryKind.Secret,
            Scope: ConfigurationScope.Stack,
            ResourceId: stackId,
            Value: null,
            SecretId: secretId,
            SecretDeliveryMode: SecretDeliveryMode.EnvironmentVariable);
        var secret = new SecretDefinition(
            Name: "API_TOKEN",
            ProviderType: SecretProviderType.VaultCompatibleKvV2,
            ProviderId: providerId,
            ExternalPath: "apps/api",
            ExternalKey: "token")
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
        var resolver = CreateResolver(
            [entry],
            configureSecrets: secrets =>
            {
                secrets.Setup(x => x.GetAsync(secretId, It.IsAny<CancellationToken>())).ReturnsAsync(secret);
            },
            configureProviders: providers =>
            {
                providers.Setup(x => x.GetAsync(providerId, It.IsAny<CancellationToken>())).ReturnsAsync(provider);
            },
            configureExternalClient: external =>
            {
                external
                    .Setup(x => x.ResolveAsync(secret, provider, "plain-token", It.IsAny<CancellationToken>()))
                    .ReturnsAsync(ExternalSecretValueResult.Success("vault-token"));
            });

        var result = await resolver.ResolveAsync(ConfigurationScope.Stack, stackId, TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess(out var resolved, out var error), error?.Message);
        Assert.Equal(["API_TOKEN=vault-token"], resolved.EnvironmentVariables);
        Assert.Equal(["vault-token"], resolved.RedactionValues);
        Assert.Equal(0, resolved.VariableCount);
        Assert.Equal(1, resolved.SecretCount);
        var snapshot = Assert.Single(resolved.SnapshotEntries);
        Assert.Equal("API_TOKEN", snapshot.Name);
        Assert.Equal("********", snapshot.Value);
        Assert.Equal(secretId, snapshot.SecretId);
        Assert.Equal("API_TOKEN", snapshot.SecretName);
        Assert.Equal(SecretProviderType.VaultCompatibleKvV2, snapshot.SecretProviderType);
        Assert.Equal("vault", snapshot.SecretProviderName);
        Assert.Equal("apps/api", snapshot.ExternalPath);
        Assert.Equal("token", snapshot.ExternalKey);
        Assert.Equal(SecretDeliveryMode.EnvironmentVariable, snapshot.SecretDeliveryMode);
    }

    [Fact]
    public async Task ResolveAsync_Should_Return_Safe_Failure_When_Internal_Secret_Cannot_Be_Decrypted()
    {
        var deploymentId = Guid.CreateVersion7();
        var secretId = Guid.CreateVersion7();
        var entry = new ConfigurationEntry(
            Name: "API_KEY",
            Kind: ConfigurationEntryKind.Secret,
            Scope: ConfigurationScope.Deployment,
            ResourceId: deploymentId,
            Value: null,
            SecretId: secretId,
            SecretDeliveryMode: SecretDeliveryMode.EnvironmentVariable);
        var secret = new SecretDefinition(
            Name: "API_KEY",
            ProviderType: SecretProviderType.InternalEncrypted)
        {
            Id = secretId
        };
        var encryptedSecret = new InternalSecretValue(secretId, "corrupt-secret-payload");
        var resolver = CreateResolver(
            [entry],
            configureSecrets: secrets =>
            {
                secrets.Setup(x => x.GetAsync(secretId, It.IsAny<CancellationToken>())).ReturnsAsync(secret);
                secrets.Setup(x => x.GetInternalValueAsync(secretId, It.IsAny<CancellationToken>())).ReturnsAsync(encryptedSecret);
            },
            unprotectException: new FormatException("raw payload parse failure"));

        var result = await resolver.ResolveAsync(ConfigurationScope.Deployment, deploymentId, TestContext.Current.CancellationToken);

        Assert.False(result.IsSuccess(out _, out var error));
        Assert.Equal("Secret API_KEY could not be decrypted.", error.Message);
        Assert.DoesNotContain("raw payload", error.Message, StringComparison.Ordinal);
        Assert.DoesNotContain("corrupt-secret-payload", error.Message, StringComparison.Ordinal);
    }

    [Fact]
    public async Task ResolveAsync_Should_Return_Safe_Failure_When_Provider_Token_Cannot_Be_Decrypted()
    {
        var stackId = Guid.CreateVersion7();
        var secretId = Guid.CreateVersion7();
        var providerId = Guid.CreateVersion7();
        var entry = new ConfigurationEntry(
            Name: "API_TOKEN",
            Kind: ConfigurationEntryKind.Secret,
            Scope: ConfigurationScope.Stack,
            ResourceId: stackId,
            Value: null,
            SecretId: secretId,
            SecretDeliveryMode: SecretDeliveryMode.EnvironmentVariable);
        var secret = new SecretDefinition(
            Name: "API_TOKEN",
            ProviderType: SecretProviderType.VaultCompatibleKvV2,
            ProviderId: providerId,
            ExternalPath: "apps/api",
            ExternalKey: "token")
        {
            Id = secretId
        };
        var provider = new SecretProvider(
            "vault",
            SecretProviderType.VaultCompatibleKvV2,
            new VaultKvV2SecretProviderConfiguration("https://vault.local", "secret", "corrupt-provider-token"))
        {
            Id = providerId
        };
        var resolver = CreateResolver(
            [entry],
            configureSecrets: secrets =>
            {
                secrets.Setup(x => x.GetAsync(secretId, It.IsAny<CancellationToken>())).ReturnsAsync(secret);
            },
            configureProviders: providers =>
            {
                providers.Setup(x => x.GetAsync(providerId, It.IsAny<CancellationToken>())).ReturnsAsync(provider);
            },
            unprotectException: new FormatException("raw token parse failure"));

        var result = await resolver.ResolveAsync(ConfigurationScope.Stack, stackId, TestContext.Current.CancellationToken);

        Assert.False(result.IsSuccess(out _, out var error));
        Assert.Equal("Secret provider token for API_TOKEN could not be decrypted.", error.Message);
        Assert.DoesNotContain("raw token", error.Message, StringComparison.Ordinal);
        Assert.DoesNotContain("corrupt-provider-token", error.Message, StringComparison.Ordinal);
    }


    [Fact]
    public void SecretRedactor_Should_Redact_Resolved_Secret_Values()
    {
        var redactor = new SecretRedactor();

        var result = redactor.Redact("token=plain-secret other=visible plain-secret", ["plain-secret"]);

        Assert.Equal("token=******** other=visible ********", result);
    }

    [Fact]
    public void SecretRedactor_Should_Redact_Longest_Secret_First()
    {
        var redactor = new SecretRedactor();

        var result = redactor.Redact("token=abc123 fallback=abc", ["abc", "abc123"]);

        Assert.Equal("token=******** fallback=********", result);
    }

    [Fact]
    public void SelectEntries_Should_Filter_Environment_Snapshots_And_Redaction_Values()
    {
        var resourceId = Guid.CreateVersion7();
        var configuration = new ResolvedConfiguration(
            EnvironmentVariables: ["APP_MODE=prod", "API_KEY=plain-secret", "UNUSED_SECRET=unused-secret"],
            Entries:
            [
                new ResolvedConfigurationEntry("APP_MODE", ConfigurationEntryKind.Variable, "prod"),
                new ResolvedConfigurationEntry(
                    "API_KEY",
                    ConfigurationEntryKind.Secret,
                    "plain-secret",
                    SecretDeliveryMode: SecretDeliveryMode.EnvironmentVariable),
                new ResolvedConfigurationEntry(
                    "UNUSED_SECRET",
                    ConfigurationEntryKind.Secret,
                    "unused-secret",
                    SecretDeliveryMode: SecretDeliveryMode.EnvironmentVariable)
            ],
            RedactionValues: ["plain-secret", "unused-secret"],
            VariableCount: 1,
            SecretCount: 2)
        {
            SnapshotEntries =
            [
                new ConfigurationSnapshotEntry(
                    Name: "APP_MODE",
                    Kind: ConfigurationEntryKind.Variable,
                    Scope: ConfigurationScope.Stack,
                    ResourceId: resourceId,
                    Value: "prod",
                    SecretId: null,
                    SecretName: null,
                    SecretProviderType: null,
                    SecretProviderName: null,
                    ExternalPath: null,
                    ExternalKey: null,
                    ExternalVersion: null,
                    SecretDeliveryMode: null,
                    TargetPath: null),
                new ConfigurationSnapshotEntry(
                    Name: "API_KEY",
                    Kind: ConfigurationEntryKind.Secret,
                    Scope: ConfigurationScope.Stack,
                    ResourceId: resourceId,
                    Value: "********",
                    SecretId: Guid.CreateVersion7(),
                    SecretName: "api-key",
                    SecretProviderType: SecretProviderType.InternalEncrypted,
                    SecretProviderName: null,
                    ExternalPath: null,
                    ExternalKey: null,
                    ExternalVersion: null,
                    SecretDeliveryMode: SecretDeliveryMode.EnvironmentVariable,
                    TargetPath: null),
                new ConfigurationSnapshotEntry(
                    Name: "UNUSED_SECRET",
                    Kind: ConfigurationEntryKind.Secret,
                    Scope: ConfigurationScope.Stack,
                    ResourceId: resourceId,
                    Value: "********",
                    SecretId: Guid.CreateVersion7(),
                    SecretName: "unused-secret",
                    SecretProviderType: SecretProviderType.InternalEncrypted,
                    SecretProviderName: null,
                    ExternalPath: null,
                    ExternalKey: null,
                    ExternalVersion: null,
                    SecretDeliveryMode: SecretDeliveryMode.EnvironmentVariable,
                    TargetPath: null)
            ]
        };

        var selected = configuration.SelectEntries(["APP_MODE", "API_KEY"]);

        Assert.Equal(["APP_MODE=prod", "API_KEY=plain-secret"], selected.EnvironmentVariables);
        Assert.Equal(["plain-secret"], selected.RedactionValues);
        Assert.Equal(["APP_MODE", "API_KEY"], selected.SnapshotEntries.Select(entry => entry.Name));
        Assert.Equal(1, selected.VariableCount);
        Assert.Equal(1, selected.SecretCount);
    }

    [Fact]
    public async Task ResolveAsync_Should_Resolve_Stack_Mounted_File_Secrets_Without_Environment_Variable()
    {
        var stackId = Guid.CreateVersion7();
        var secretId = Guid.CreateVersion7();
        var entry = new ConfigurationEntry(
            Name: "API_KEY",
            Kind: ConfigurationEntryKind.Secret,
            Scope: ConfigurationScope.Stack,
            ResourceId: stackId,
            Value: null,
            SecretId: secretId,
            SecretDeliveryMode: SecretDeliveryMode.MountedFile,
            TargetPath: "/run/secrets/api_key");
        var secret = new SecretDefinition(
            Name: "API_KEY",
            ProviderType: SecretProviderType.InternalEncrypted)
        {
            Id = secretId
        };
        var encryptedSecret = new InternalSecretValue(secretId, "encrypted");
        var resolver = CreateResolver(
            [entry],
            configureSecrets: secrets =>
            {
                secrets.Setup(x => x.GetAsync(secretId, It.IsAny<CancellationToken>())).ReturnsAsync(secret);
                secrets.Setup(x => x.GetInternalValueAsync(secretId, It.IsAny<CancellationToken>())).ReturnsAsync(encryptedSecret);
            },
            unprotectedSecretValue: "plain-secret");

        var result = await resolver.ResolveAsync(ConfigurationScope.Stack, stackId, TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess(out var resolved, out var error), error?.Message);
        Assert.Empty(resolved.EnvironmentVariables);
        Assert.Equal(["plain-secret"], resolved.RedactionValues);
        var entryResult = Assert.Single(resolved.Entries);
        Assert.Equal("API_KEY", entryResult.Name);
        Assert.Equal("plain-secret", entryResult.Value);
        Assert.Equal(SecretDeliveryMode.MountedFile, entryResult.SecretDeliveryMode);
        Assert.Equal("/run/secrets/api_key", entryResult.TargetPath);
        var snapshot = Assert.Single(resolved.SnapshotEntries);
        Assert.Equal(SecretDeliveryMode.MountedFile, snapshot.SecretDeliveryMode);
        Assert.Equal("/run/secrets/api_key", snapshot.TargetPath);
    }

    [Fact]
    public async Task ResolveAsync_Should_Reject_Native_Platform_Secret_Delivery_Mode()
    {
        var stackId = Guid.CreateVersion7();
        var entry = new ConfigurationEntry(
            Name: "API_KEY",
            Kind: ConfigurationEntryKind.Secret,
            Scope: ConfigurationScope.Stack,
            ResourceId: stackId,
            Value: null,
            SecretId: Guid.CreateVersion7(),
            SecretDeliveryMode: SecretDeliveryMode.NativePlatformSecret);
        var resolver = CreateResolver([entry]);

        var result = await resolver.ResolveAsync(ConfigurationScope.Stack, stackId, TestContext.Current.CancellationToken);

        Assert.False(result.IsSuccess(out _, out var error));
        Assert.Contains("Native platform secret delivery is not supported", error.Message);
    }

    [Fact]
    public async Task ResolveAsync_Should_Reject_Mounted_File_Secrets_For_Deployments()
    {
        var deploymentId = Guid.CreateVersion7();
        var entry = new ConfigurationEntry(
            Name: "API_KEY",
            Kind: ConfigurationEntryKind.Secret,
            Scope: ConfigurationScope.Deployment,
            ResourceId: deploymentId,
            Value: null,
            SecretId: Guid.CreateVersion7(),
            SecretDeliveryMode: SecretDeliveryMode.MountedFile,
            TargetPath: "/run/secrets/api_key");
        var resolver = CreateResolver([entry]);

        var result = await resolver.ResolveAsync(ConfigurationScope.Deployment, deploymentId, TestContext.Current.CancellationToken);

        Assert.False(result.IsSuccess(out _, out var error));
        Assert.Equal("Secret API_KEY uses mounted-file delivery, which is only supported for stack-scoped entries.", error.Message);
    }

    [Fact]
    public async Task ResolveAsync_Should_Reject_Global_Mounted_File_Secrets_For_Stacks()
    {
        var stackId = Guid.CreateVersion7();
        var entry = new ConfigurationEntry(
            Name: "API_KEY",
            Kind: ConfigurationEntryKind.Secret,
            Scope: ConfigurationScope.Global,
            ResourceId: null,
            Value: null,
            SecretId: Guid.CreateVersion7(),
            SecretDeliveryMode: SecretDeliveryMode.MountedFile,
            TargetPath: "/run/secrets/api_key");
        var resolver = CreateResolver([entry]);

        var result = await resolver.ResolveAsync(ConfigurationScope.Stack, stackId, TestContext.Current.CancellationToken);

        Assert.False(result.IsSuccess(out _, out var error));
        Assert.Equal("Secret API_KEY uses mounted-file delivery, which is only supported for stack-scoped entries.", error.Message);
    }

    [Fact]
    public void ActivitySerialization_Should_Not_Persist_Secret_Plaintext()
    {
        ActivityEventInfo info = new DeploymentApplied(
            new DeploymentSnapshot(Guid.CreateVersion7(), "api", Guid.CreateVersion7()),
            new DeploymentResultSnapshot(
                ContainerIds: ["container-1"],
                Message: "Deployment applied.",
                Configuration:
                [
                    new ConfigurationSnapshotEntry(
                        Name: "DATABASE_PASSWORD",
                        Kind: ConfigurationEntryKind.Secret,
                        Scope: ConfigurationScope.Deployment,
                        ResourceId: Guid.CreateVersion7(),
                        Value: "********",
                        SecretId: Guid.CreateVersion7(),
                        SecretName: "prod-db-password",
                        SecretProviderType: SecretProviderType.VaultCompatibleKvV2,
                        SecretProviderName: "vault",
                        ExternalPath: "apps/api/prod",
                        ExternalKey: "database_password",
                        ExternalVersion: null,
                        SecretDeliveryMode: SecretDeliveryMode.EnvironmentVariable,
                        TargetPath: null)
                ]));

        var json = JsonSerializer.Serialize(info, EventInfoJsonContext.Default.ActivityEventInfo);

        Assert.Contains("DATABASE_PASSWORD", json, StringComparison.Ordinal);
        Assert.Contains("prod-db-password", json, StringComparison.Ordinal);
        Assert.Contains("********", json, StringComparison.Ordinal);
        Assert.DoesNotContain("plain-secret", json, StringComparison.Ordinal);
        Assert.DoesNotContain("vault-token", json, StringComparison.Ordinal);
    }

    private static ConfigurationResolver CreateResolver(
        IReadOnlyList<ConfigurationEntry> entries,
        Action<Mock<ISecretDefinitionRepository>>? configureSecrets = null,
        Action<Mock<ISecretProviderRepository>>? configureProviders = null,
        Action<Mock<IExternalSecretProviderClient>>? configureExternalClient = null,
        string unprotectedSecretValue = "plain-token",
        Exception? unprotectException = null)
    {
        var configurationEntries = new Mock<IConfigurationEntryRepository>();
        configurationEntries
            .Setup(x => x.GetEffectiveEntriesAsync(It.IsAny<ConfigurationScope>(), It.IsAny<Guid>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(entries);

        var secrets = new Mock<ISecretDefinitionRepository>();
        configureSecrets?.Invoke(secrets);

        var providers = new Mock<ISecretProviderRepository>();
        configureProviders?.Invoke(providers);

        var uow = new Mock<IUnitOfWork>();
        uow.Setup(x => x.ConfigurationEntries).Returns(configurationEntries.Object);
        uow.Setup(x => x.SecretDefinitions).Returns(secrets.Object);
        uow.Setup(x => x.SecretProviders).Returns(providers.Object);

        var externalClient = new Mock<IExternalSecretProviderClient>();
        configureExternalClient?.Invoke(externalClient);

        var services = new ServiceCollection()
            .AddSingleton(uow.Object)
            .BuildServiceProvider();

        return new ConfigurationResolver(
            services.GetRequiredService<IServiceScopeFactory>(),
            new TestSecretValueProtector(unprotectedSecretValue, unprotectException),
            externalClient.Object);
    }

    private sealed class TestSecretValueProtector(string unprotectedValue, Exception? unprotectException = null) : ISecretValueProtector
    {
        public string Protect(string value) => value;

        public string Unprotect(string protectedValue)
        {
            if (unprotectException is not null)
                throw unprotectException;

            return unprotectedValue;
        }
    }
}
