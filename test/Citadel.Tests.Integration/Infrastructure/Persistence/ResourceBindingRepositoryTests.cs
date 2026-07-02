using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.ResourceBindings;
using Microsoft.Extensions.DependencyInjection;

namespace Tests.Integration.Infrastructure.Persistence;

public sealed class ResourceBindingRepositoryTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    [Fact]
    public async Task SecretProviderUsage_Should_Only_Count_Bound_External_Secrets()
    {
        var provider = new SecretProvider(
            "vault-usage",
            SecretProviderType.VaultCompatibleKvV2,
            new VaultKvV2SecretProviderConfiguration("https://vault.local", "secret", "protected"));
        var secret = new SecretDefinition(
            "API_KEY_USAGE",
            SecretProviderType.VaultCompatibleKvV2,
            provider.Id,
            "apps/api/prod",
            "api_key");

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        await uow.SecretProviders.AddAsync(provider, TestContext.Current.CancellationToken);
        await uow.SecretDefinitions.AddAsync(secret, value: null, TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);

        var usedBeforeBinding = await uow.SecretProviders.IsUsedByResourceBindingAsync(
            provider.Id,
            TestContext.Current.CancellationToken);

        await uow.ResourceBindings.AddAsync(
            new ResourceBinding(
                Name: "API_KEY_USAGE",
                Kind: ResourceBindingKind.Secret,
                Scope: ResourceBindingScope.Stack,
                ResourceId: Guid.CreateVersion7(),
                Value: null,
                SecretId: secret.Id,
                SecretDeliveryMode: SecretDeliveryMode.EnvironmentVariable),
            TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);

        var usedAfterBinding = await uow.SecretProviders.IsUsedByResourceBindingAsync(
            provider.Id,
            TestContext.Current.CancellationToken);

        Assert.False(usedBeforeBinding);
        Assert.True(usedAfterBinding);
    }

    [Fact]
    public async Task ReplaceResourceEntriesAsync_Should_Delete_And_Bulk_Insert_Resource_Entries()
    {
        var stackId = Guid.CreateVersion7();
        var secretId = Guid.CreateVersion7();
        var original = new ResourceBinding(
            Name: "APP_MODE",
            Kind: ResourceBindingKind.Variable,
            Scope: ResourceBindingScope.Stack,
            ResourceId: stackId,
            Value: "old",
            SecretId: null);
        var replacementVariable = new ResourceBinding(
            Name: "APP_MODE",
            Kind: ResourceBindingKind.Variable,
            Scope: ResourceBindingScope.Stack,
            ResourceId: stackId,
            Value: "new",
            SecretId: null);
        var replacementSecret = new ResourceBinding(
            Name: "API_KEY",
            Kind: ResourceBindingKind.Secret,
            Scope: ResourceBindingScope.Stack,
            ResourceId: stackId,
            Value: null,
            SecretId: secretId,
            SecretDeliveryMode: SecretDeliveryMode.EnvironmentVariable);

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        await uow.SecretDefinitions.AddAsync(
            new SecretDefinition("API_KEY", SecretProviderType.InternalEncrypted) { Id = secretId },
            new InternalSecretValue(secretId, "encrypted"),
            TestContext.Current.CancellationToken);
        await uow.ResourceBindings.ReplaceResourceEntriesAsync(
            ResourceBindingScope.Stack,
            stackId,
            [original],
            TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);

        var affected = await uow.ResourceBindings.ReplaceResourceEntriesAsync(
            ResourceBindingScope.Stack,
            stackId,
            [replacementVariable, replacementSecret],
            TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);

        var entries = (await uow.ResourceBindings.GetEffectiveEntriesAsync(
            ResourceBindingScope.Stack,
            stackId,
            TestContext.Current.CancellationToken)).ToArray();

        Assert.Equal(3, affected);
        Assert.Equal(2, entries.Length);
        Assert.Contains(entries, x => x.Name == "APP_MODE" && x.Value == "new");
        Assert.Contains(entries, x => x.Name == "API_KEY" && x.SecretId == secretId);
    }
}
