using System.Text;
using Domain;
using Domain.Contracts.Interfaces;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;

namespace Tests.Integration.Application.Features.Alerters;

public class AlertRuleCreateTests : IntegrationTestBase
{
    [Fact]
    public async Task Create_NonThreshold_AlertRule_ReturnsSuccess()
    {
        // Arrange
        var createJson = """
        {
          "type": "PlatformUnreachable",
          "severity": "Critical",
          "cooldownSeconds": 300,
          "isEnabled": true,
          "scope": "All"
        }
        """;
        var content = new StringContent(createJson, Encoding.UTF8, "application/json");

        // Act
        var response = await Client.PostAsync("/api/v1/alerters/rules", content, cancellationToken: TestContext.Current.CancellationToken);

        // Assert
        response.EnsureSuccessStatusCode();
        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var rules = await uow.AlertRules.GetAllAsync(TestContext.Current.CancellationToken);

        Assert.Contains(rules, r => r.Type == AlertType.PlatformUnreachable);
        await VerifyJson(responseBody);
    }

    [Fact]
    public async Task Create_Threshold_AlertRule_ReturnsSuccess()
    {
        // Arrange
        var createJson = """
        {
          "type": "PlatformCpuHigh",
          "severity": "Warning",
          "cooldownSeconds": 60,
          "isEnabled": true,
          "scope": "All",
          "requiredMatches": 3,
          "threshold": 85.0
        }
        """;
        var content = new StringContent(createJson, Encoding.UTF8, "application/json");

        // Act
        var response = await Client.PostAsync("/api/v1/alerters/rules", content, cancellationToken: TestContext.Current.CancellationToken);

        // Assert
        response.EnsureSuccessStatusCode();
        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var rules = await uow.AlertRules.GetAllAsync(TestContext.Current.CancellationToken);

        Assert.Contains(rules, r => r.Type == AlertType.PlatformCpuHigh && r.Threshold == 85.0);
        await VerifyJson(responseBody);
    }

    [Fact]
    public async Task Create_AlertRule_With_Channels_ReturnsSuccess()
    {
        // Arrange
        var createJson = """
        {
          "type": "PlatformUnreachable",
          "severity": "Critical",
          "cooldownSeconds": 120,
          "isEnabled": true,
          "scope": "All",
          "channels": [
            {
              "alertDestination": "Slack",
              "url": "https://hooks.slack.com/services/test",
              "isActive": true
            }
          ]
        }
        """;
        var content = new StringContent(createJson, Encoding.UTF8, "application/json");

        // Act
        var response = await Client.PostAsync("/api/v1/alerters/rules", content, cancellationToken: TestContext.Current.CancellationToken);

        // Assert
        response.EnsureSuccessStatusCode();
        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var rules = await uow.AlertRules.GetAllAsync(TestContext.Current.CancellationToken);

        var rule = Assert.Single(rules, r => r.Type == AlertType.PlatformUnreachable && r.CooldownSeconds == 120);
        Assert.Single(rule.Channels);
        await VerifyJson(responseBody);
    }

    [Fact]
    public async Task Create_AlertRule_With_Invalid_CooldownSeconds_Returns_BadRequest()
    {
        // Arrange — cooldown below minimum (10)
        var createJson = """
        {
          "type": "PlatformUnreachable",
          "severity": "Warning",
          "cooldownSeconds": 5,
          "isEnabled": true,
          "scope": "All"
        }
        """;
        var content = new StringContent(createJson, Encoding.UTF8, "application/json");

        // Act
        var response = await Client.PostAsync("/api/v1/alerters/rules", content, cancellationToken: TestContext.Current.CancellationToken);

        // Assert
        Assert.Equal(System.Net.HttpStatusCode.BadRequest, response.StatusCode);
        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        await VerifyJson(responseBody);
    }

    [Fact]
    public async Task Create_Threshold_AlertRule_Without_RequiredMatches_Returns_BadRequest()
    {
        // Arrange — PlatformCpuHigh is a threshold type and requires RequiredMatches + Threshold
        var createJson = """
        {
          "type": "PlatformCpuHigh",
          "severity": "Warning",
          "cooldownSeconds": 60,
          "isEnabled": true,
          "scope": "All",
          "threshold": 85.0
        }
        """;
        var content = new StringContent(createJson, Encoding.UTF8, "application/json");

        // Act
        var response = await Client.PostAsync("/api/v1/alerters/rules", content, cancellationToken: TestContext.Current.CancellationToken);

        // Assert
        Assert.Equal(System.Net.HttpStatusCode.BadRequest, response.StatusCode);
        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        await VerifyJson(responseBody);
    }

    [Fact]
    public async Task Create_AlertRule_With_Scope_Specific_But_No_LimitedTo_Returns_BadRequest()
    {
        // Arrange
        var createJson = """
        {
          "type": "PlatformUnreachable",
          "severity": "Warning",
          "cooldownSeconds": 60,
          "isEnabled": true,
          "scope": "Specific"
        }
        """;
        var content = new StringContent(createJson, Encoding.UTF8, "application/json");

        // Act
        var response = await Client.PostAsync("/api/v1/alerters/rules", content, cancellationToken: TestContext.Current.CancellationToken);

        // Assert
        Assert.Equal(System.Net.HttpStatusCode.BadRequest, response.StatusCode);
        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        await VerifyJson(responseBody);
    }

    [Fact]
    public async Task Create_NonThreshold_AlertRule_With_Threshold_Returns_BadRequest()
    {
        // Arrange — non-threshold type must not define Threshold or RequiredMatches
        var createJson = """
        {
          "type": "PlatformUnreachable",
          "severity": "Warning",
          "cooldownSeconds": 60,
          "isEnabled": true,
          "scope": "All",
          "requiredMatches": 3,
          "threshold": 85.0
        }
        """;
        var content = new StringContent(createJson, Encoding.UTF8, "application/json");

        // Act
        var response = await Client.PostAsync("/api/v1/alerters/rules", content, cancellationToken: TestContext.Current.CancellationToken);

        // Assert
        Assert.Equal(System.Net.HttpStatusCode.BadRequest, response.StatusCode);
        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        await VerifyJson(responseBody);
    }
}
