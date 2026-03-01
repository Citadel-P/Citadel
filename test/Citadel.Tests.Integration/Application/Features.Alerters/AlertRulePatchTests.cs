using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Alerts;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;
using System.Text;

namespace Tests.Integration.Application.Features.Alerters;

public class AlertRulePatchTests : IntegrationTestBase
{
    private Guid ruleId;

    protected override async ValueTask SeedDbAsync(IUnitOfWork uow)
    {
        var rule = new AlertRule(
            type: AlertType.PlatformUnreachable,
            severity: AlertSeverity.Warning,
            cooldownSeconds: 300,
            isEnabled: true,
            createdByActorId: Constants.SystemId);

        await uow.AlertRules.AddAlertRuleAsync(rule, TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);
        ruleId = rule.Id;
    }

    [Fact]
    public async Task Patch_AlertRule_Should_Apply_MergePatch()
    {
        // Arrange
        var patchJson = """
        {
          "severity": "Critical",
          "cooldownSeconds": 600,
          "isEnabled": false
        }
        """;
        var content = new StringContent(patchJson, Encoding.UTF8, "application/merge-patch+json");

        // Act
        var response = await Client.PatchAsync($"/api/v1/alerters/rules/{ruleId}", content, cancellationToken: TestContext.Current.CancellationToken);
        response.EnsureSuccessStatusCode();

        // Assert
        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var updated = await uow.AlertRules.GetByIdAsync(ruleId, TestContext.Current.CancellationToken);

        Assert.NotNull(updated);
        Assert.Equal(AlertSeverity.Critical, updated.Severity);
        Assert.Equal(600, updated.CooldownSeconds);
        Assert.False(updated.IsEnabled);
        await VerifyJson(responseBody);
    }

    [Fact]
    public async Task Patch_AlertRule_With_Invalid_CooldownSeconds_Should_Return_BadRequest()
    {
        // Arrange — cooldown below minimum
        var patchJson = """
        {
          "cooldownSeconds": 5
        }
        """;
        var content = new StringContent(patchJson, Encoding.UTF8, "application/merge-patch+json");

        // Act
        var response = await Client.PatchAsync($"/api/v1/alerters/rules/{ruleId}", content, cancellationToken: TestContext.Current.CancellationToken);

        // Assert
        Assert.Equal(System.Net.HttpStatusCode.BadRequest, response.StatusCode);
        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        await VerifyJson(responseBody);
    }

    [Fact]
    public async Task Patch_NonExistent_AlertRule_Should_Return_NotFound()
    {
        // Arrange
        var nonExistentId = Guid.NewGuid();
        var patchJson = """
        {
          "severity": "Critical"
        }
        """;
        var content = new StringContent(patchJson, Encoding.UTF8, "application/merge-patch+json");

        // Act
        var response = await Client.PatchAsync($"/api/v1/alerters/rules/{nonExistentId}", content, cancellationToken: TestContext.Current.CancellationToken);

        // Assert
        Assert.Equal(System.Net.HttpStatusCode.NotFound, response.StatusCode);
        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        await VerifyJson(responseBody);
    }
}
