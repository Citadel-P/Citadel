using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Alerts;
using Microsoft.Extensions.DependencyInjection;

namespace Tests.Integration.Application.Features.Alerters;

public sealed class AlertRuleSeedTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    [Theory]
    [InlineData(AlertType.StackGitUpdateAvailable)]
    [InlineData(AlertType.StackGitAutoUpdated)]
    [InlineData(AlertType.StackGitAutoDeployFailed)]
    public async Task Default_AlertRules_Should_Include_GitStack_Events(AlertType type)
    {
        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();

        var rules = await uow.AlertRules.GetAllAsync(TestContext.Current.CancellationToken);
        var rule = Assert.Single(rules, rule => rule.Type == type);

        Assert.Equal(AlertRuleStatus.Enabled, rule.Status);
        Assert.Equal(AlertResourceType.Stack, AlertTypeMetadata.GetResourceType(rule.Type));
    }
}
