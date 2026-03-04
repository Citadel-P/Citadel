using System.Text;
using Application.Services.Alerts;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Alerts;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;

namespace Tests.Integration.Application.Features.Alerters;

public class AlertRuleCreateTests : IntegrationTestBase
{
    private Guid channelId;

    protected override async ValueTask SeedDbAsync(IUnitOfWork uow)
    {
        var channel = new AlertChannel(
            alertDestination: AlertDestination.Slack,
            url: "https://hooks.slack.com/services/test",
            isActive: true,
            createdByActorId: Constants.SystemId);

        await uow.AlertRules.AddChannelAsync(channel, TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);
        channelId = channel.Id;
    }

    [Fact]
    public async Task Create_NonThreshold_AlertRule_ReturnsSuccess()
    {
        // Arrange
        var createJson = """
        {
          "type": "PlatformUnreachable",
          "severity": "Critical",
          "cooldownSeconds": 300,
          "isEnabled": true
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
        var cache = scope.ServiceProvider.GetRequiredService<IAlertRuleProvider>();

        var rule = Assert.Single(rules.Where(r =>
            r.Type == AlertType.PlatformUnreachable &&
            r.Severity == AlertSeverity.Critical &&
            r.CooldownSeconds == 300 &&
            r.IsEnabled));
        Assert.Contains(cache.Current.Get(AlertType.PlatformUnreachable), r => r.Id == rule.Id);
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
        var cache = scope.ServiceProvider.GetRequiredService<IAlertRuleProvider>();

        var rule = Assert.Single(rules.Where(r => r.Type == AlertType.PlatformCpuHigh && r.Threshold == 85.0));
        Assert.Contains(cache.Current.Get(AlertType.PlatformCpuHigh), r => r.Id == rule.Id);
        await VerifyJson(responseBody);
    }

    [Fact]
    public async Task Create_AlertRule_With_Channels_ReturnsSuccess()
    {
        // Arrange
        var createJson = $$"""
        {
          "type": "PlatformUnreachable",
          "severity": "Critical",
          "cooldownSeconds": 120,
          "isEnabled": true,
          "channelIds": [
            "{{channelId}}"
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
        var cache = scope.ServiceProvider.GetRequiredService<IAlertRuleProvider>();

        var rule = Assert.Single(rules, r => r.Type == AlertType.PlatformUnreachable && r.CooldownSeconds == 120);
        Assert.Contains(channelId, rule.ChannelIds);
        Assert.Contains(cache.Current.Get(AlertType.PlatformUnreachable), r => r.Id == rule.Id && r.ChannelIds.Contains(channelId));
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
          "isEnabled": true
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
    public async Task Create_NonThreshold_AlertRule_With_Threshold_Returns_BadRequest()
    {
        // Arrange — non-threshold type must not define Threshold or RequiredMatches
        var createJson = """
        {
          "type": "PlatformUnreachable",
          "severity": "Warning",
          "cooldownSeconds": 60,
          "isEnabled": true,
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

    [Fact]
    public async Task Create_AlertChannel_ReturnsSuccess()
    {
        // Arrange
        await using var beforeScope = Services.CreateAsyncScope();
        var beforeCache = beforeScope.ServiceProvider.GetRequiredService<IAlertRuleProvider>();
        var beforeCount = GetCachedRuleCount(beforeCache);

        var createJson = """
        {
          "alertDestination": "Slack",
          "url": "https://hooks.slack.com/services/test",
          "isActive": true
        }
        """;
        var content = new StringContent(createJson, Encoding.UTF8, "application/json");

        // Act
        var response = await Client.PostAsync("/api/v1/alerters/channels", content, cancellationToken: TestContext.Current.CancellationToken);

        // Assert
        response.EnsureSuccessStatusCode();

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var cache = scope.ServiceProvider.GetRequiredService<IAlertRuleProvider>();
        var channels = await uow.AlertRules.GetAllChannelsAsync(TestContext.Current.CancellationToken);
        Assert.Contains(channels, c => c.AlertDestination == AlertDestination.Slack && c.Url == "https://hooks.slack.com/services/test");
        Assert.Equal(beforeCount, GetCachedRuleCount(cache));
    }

    [Fact]
    public async Task Create_AlertChannel_With_Empty_Url_Returns_BadRequest()
    {
        // Arrange
        await using var beforeScope = Services.CreateAsyncScope();
        var beforeCache = beforeScope.ServiceProvider.GetRequiredService<IAlertRuleProvider>();
        var beforeCount = GetCachedRuleCount(beforeCache);

        var createJson = """
        {
          "alertDestination": "Slack",
          "url": "",
          "isActive": true
        }
        """;
        var content = new StringContent(createJson, Encoding.UTF8, "application/json");

        // Act
        var response = await Client.PostAsync("/api/v1/alerters/channels", content, cancellationToken: TestContext.Current.CancellationToken);

        // Assert
        Assert.Equal(System.Net.HttpStatusCode.BadRequest, response.StatusCode);

        await using var afterScope = Services.CreateAsyncScope();
        var afterCache = afterScope.ServiceProvider.GetRequiredService<IAlertRuleProvider>();
        Assert.Equal(beforeCount, GetCachedRuleCount(afterCache));
    }

    private static int GetCachedRuleCount(IAlertRuleProvider cache)
        => cache.Current.ByType.SelectMany(x => x.Value).Count();
}
