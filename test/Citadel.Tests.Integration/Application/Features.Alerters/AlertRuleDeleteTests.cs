using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Alerts;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;
using System.Text;

namespace Tests.Integration.Application.Features.Alerters;

public class AlertRuleDeleteTests : IntegrationTestBase
{
    private Guid ruleId;

    protected override async ValueTask SeedDbAsync(IUnitOfWork uow)
    {
        var rule = new AlertRule(
            type: AlertType.PlatformUnreachable,
            severity: AlertSeverity.Critical,
            cooldownSeconds: 300,
            isEnabled: true,
            scope: AlertScope.All,
            createdByActorId: Constants.SystemId);

        await uow.AlertRules.AddAlertRuleAsync(rule, TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);
        ruleId = rule.Id;
    }

    [Fact]
    public async Task Delete_AlertRule_ReturnsSuccess()
    {
        // Arrange
        var content = $$"""
        {
            "ids": ["{{ruleId}}"]
        }
        """;

        // Act
        var request = new HttpRequestMessage(HttpMethod.Delete, "/api/v1/alerters/rules")
        {
            Content = new StringContent(content, Encoding.UTF8, "application/json")
        };

        var response = await Client.SendAsync(request, TestContext.Current.CancellationToken);

        // Assert
        response.EnsureSuccessStatusCode();

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var rules = await uow.AlertRules.GetAllAsync(TestContext.Current.CancellationToken);

        Assert.DoesNotContain(rules, r => r.Id == ruleId);
        Assert.Equal(string.Empty, await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken));
    }

    [Fact]
    public async Task Delete_NonExistent_AlertRule_Returns_NotFound()
    {
        // Arrange
        var nonExistentId = Guid.NewGuid();
        var content = $$"""
        {
            "ids": ["{{nonExistentId}}"]
        }
        """;

        // Act
        var request = new HttpRequestMessage(HttpMethod.Delete, "/api/v1/alerters/rules")
        {
            Content = new StringContent(content, Encoding.UTF8, "application/json")
        };

        var response = await Client.SendAsync(request, TestContext.Current.CancellationToken);

        // Assert
        Assert.Equal(System.Net.HttpStatusCode.NotFound, response.StatusCode);
    }
}
