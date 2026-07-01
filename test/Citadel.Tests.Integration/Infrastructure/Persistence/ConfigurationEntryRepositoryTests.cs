using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Configuration;
using Microsoft.Extensions.DependencyInjection;

namespace Tests.Integration.Infrastructure.Persistence;

public sealed class ConfigurationEntryRepositoryTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    [Fact]
    public async Task ReplaceResourceEntriesAsync_Should_Delete_And_Bulk_Insert_Resource_Entries()
    {
        var stackId = Guid.CreateVersion7();
        var secretId = Guid.CreateVersion7();
        var original = new ConfigurationEntry(
            Name: "APP_MODE",
            Kind: ConfigurationEntryKind.Variable,
            Scope: ConfigurationScope.Stack,
            ResourceId: stackId,
            Value: "old",
            SecretId: null);
        var replacementVariable = new ConfigurationEntry(
            Name: "APP_MODE",
            Kind: ConfigurationEntryKind.Variable,
            Scope: ConfigurationScope.Stack,
            ResourceId: stackId,
            Value: "new",
            SecretId: null);
        var replacementSecret = new ConfigurationEntry(
            Name: "API_KEY",
            Kind: ConfigurationEntryKind.Secret,
            Scope: ConfigurationScope.Stack,
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
        await uow.ConfigurationEntries.ReplaceResourceEntriesAsync(
            ConfigurationScope.Stack,
            stackId,
            [original],
            TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);

        var affected = await uow.ConfigurationEntries.ReplaceResourceEntriesAsync(
            ConfigurationScope.Stack,
            stackId,
            [replacementVariable, replacementSecret],
            TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);

        var entries = (await uow.ConfigurationEntries.GetEffectiveEntriesAsync(
            ConfigurationScope.Stack,
            stackId,
            TestContext.Current.CancellationToken)).ToArray();

        Assert.Equal(3, affected);
        Assert.Equal(2, entries.Length);
        Assert.Contains(entries, x => x.Name == "APP_MODE" && x.Value == "new");
        Assert.Contains(entries, x => x.Name == "API_KEY" && x.SecretId == secretId);
    }
}
