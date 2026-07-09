using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Alerts;
using Microsoft.Extensions.DependencyInjection;

namespace Tests.Integration.Application.Features.Alerters;

public sealed class AlertRuleSeedTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    [Theory]
    [InlineData(AlertType.StackGitUpdateAvailable, AlertResourceType.Stack)]
    [InlineData(AlertType.StackGitAutoUpdated, AlertResourceType.Stack)]
    [InlineData(AlertType.StackGitAutoDeployFailed, AlertResourceType.Stack)]
    [InlineData(AlertType.AutomationActionRunFailed, AlertResourceType.AutomationAction)]
    public async Task Default_AlertRules_Should_Include_Seeded_Events(AlertType type, AlertResourceType resourceType)
    {
        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();

        var rules = await uow.AlertRules.GetAllAsync(TestContext.Current.CancellationToken);
        var rule = Assert.Single(rules, rule => rule.Type == type);

        Assert.Equal(AlertRuleStatus.Enabled, rule.Status);
        Assert.Equal(resourceType, AlertTypeMetadata.GetResourceType(rule.Type));
    }
}
