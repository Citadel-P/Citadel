using Application.Services;
using Domain.Contracts.Interfaces;
using Domain.Entities.Configuration;
using Microsoft.Extensions.DependencyInjection;
using Moq;

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
    }

    [Fact]
    public void SecretRedactor_Should_Redact_Resolved_Secret_Values()
    {
        var redactor = new SecretRedactor();

        var result = redactor.Redact("token=plain-secret other=visible plain-secret", ["plain-secret"]);

        Assert.Equal("token=******** other=visible ********", result);
    }

    private static ConfigurationResolver CreateResolver(
        IReadOnlyList<ConfigurationEntry> entries,
        Action<Mock<ISecretDefinitionRepository>>? configureSecrets = null,
        string unprotectedSecretValue = "")
    {
        var configurationEntries = new Mock<IConfigurationEntryRepository>();
        configurationEntries
            .Setup(x => x.GetEffectiveEntriesAsync(It.IsAny<ConfigurationScope>(), It.IsAny<Guid>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(entries);

        var secrets = new Mock<ISecretDefinitionRepository>();
        configureSecrets?.Invoke(secrets);

        var uow = new Mock<IUnitOfWork>();
        uow.Setup(x => x.ConfigurationEntries).Returns(configurationEntries.Object);
        uow.Setup(x => x.SecretDefinitions).Returns(secrets.Object);

        var services = new ServiceCollection()
            .AddSingleton(uow.Object)
            .BuildServiceProvider();

        return new ConfigurationResolver(
            services.GetRequiredService<IServiceScopeFactory>(),
            new TestSecretValueProtector(unprotectedSecretValue));
    }

    private sealed class TestSecretValueProtector(string unprotectedValue) : ISecretValueProtector
    {
        public string Protect(string value) => value;

        public string Unprotect(string protectedValue) => unprotectedValue;
    }
}
