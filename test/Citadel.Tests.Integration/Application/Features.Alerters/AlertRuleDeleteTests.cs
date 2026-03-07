using Domain;
using Application.Services.Alerts;
using Domain.Contracts.Interfaces;
using Domain.Entities.Alerts;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;
using System.Text;

namespace Tests.Integration.Application.Features.Alerters;

public class AlertRuleDeleteTests : IntegrationTestBase
{
    private Guid ruleId;
    private Guid channelId;

    protected override async ValueTask SeedDbAsync(IUnitOfWork uow)
    {
        var rule = new AlertRule(
            name: "Test Rule",
            type: AlertType.PlatformUnreachable,
            severity: AlertSeverity.Critical,
            cooldownSeconds: 300,
            status: AlertRuleStatus.Enabled,
            createdByActorId: Constants.SystemId);

        await uow.AlertRules.AddAlertRuleAsync(rule, TestContext.Current.CancellationToken);

        var channel = new AlertChannel(
            alertDestination: AlertDestination.Slack,
            url: "https://hooks.slack.com/services/delete",
            isActive: true,
            createdByActorId: Constants.SystemId);
        await uow.AlertRules.AddChannelAsync(channel, TestContext.Current.CancellationToken);

        await uow.CommitAsync(TestContext.Current.CancellationToken);
        ruleId = rule.Id;
        channelId = channel.Id;
    }

    [Fact]
    public async Task Delete_AlertRule_ReturnsSuccess()
    {
        await using (var prepScope = Services.CreateAsyncScope())
        {
            var prepUow = prepScope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            var prepCache = prepScope.ServiceProvider.GetRequiredService<AlertRuleCache>();
            var rule = await prepUow.AlertRules.GetByIdAsync(ruleId, TestContext.Current.CancellationToken);
            Assert.NotNull(rule);
            prepCache.Upsert(rule);
        }

        // Arrange
        var content = $$"""
        {
            "ids": ["{{ruleId}}"]
        }
        """;

        // Act
        var request = new HttpRequestMessage(HttpMethod.Delete, "/api/v1/alertRules")
        {
            Content = new StringContent(content, Encoding.UTF8, "application/json")
        };

        var response = await Client.SendAsync(request, TestContext.Current.CancellationToken);

        // Assert
        response.EnsureSuccessStatusCode();

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var cache = scope.ServiceProvider.GetRequiredService<IAlertRuleProvider>();
        var rules = await uow.AlertRules.GetAllAsync(TestContext.Current.CancellationToken);

        Assert.DoesNotContain(rules, r => r.Id == ruleId);
        Assert.DoesNotContain(cache.Current.Get(AlertType.PlatformUnreachable), r => r.Id == ruleId);
        Assert.Equal(string.Empty, await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken));
    }

    [Fact]
    public async Task Delete_NonExistent_AlertRule_Returns_NotFound()
    {
        await using (var prepScope = Services.CreateAsyncScope())
        {
            var prepUow = prepScope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            var prepCache = prepScope.ServiceProvider.GetRequiredService<AlertRuleCache>();
            var rule = await prepUow.AlertRules.GetByIdAsync(ruleId, TestContext.Current.CancellationToken);
            Assert.NotNull(rule);
            prepCache.Upsert(rule);
        }

        // Arrange
        var nonExistentId = Guid.NewGuid();
        var content = $$"""
        {
            "ids": ["{{nonExistentId}}"]
        }
        """;

        // Act
        var request = new HttpRequestMessage(HttpMethod.Delete, "/api/v1/alertRules")
        {
            Content = new StringContent(content, Encoding.UTF8, "application/json")
        };

        var response = await Client.SendAsync(request, TestContext.Current.CancellationToken);

        // Assert
        Assert.Equal(System.Net.HttpStatusCode.NotFound, response.StatusCode);

        await using var scope = Services.CreateAsyncScope();
        var cache = scope.ServiceProvider.GetRequiredService<IAlertRuleProvider>();
        Assert.Contains(cache.Current.Get(AlertType.PlatformUnreachable), r => r.Id == ruleId);
    }

    [Fact]
    public async Task Delete_AlertChannel_ReturnsSuccess()
    {
        await using var beforeScope = Services.CreateAsyncScope();
        var beforeCache = beforeScope.ServiceProvider.GetRequiredService<IAlertRuleProvider>();
        var beforeCount = GetCachedRuleCount(beforeCache);

        // Arrange
        var content = $$"""
        {
            "ids": ["{{channelId}}"]
        }
        """;

        // Act
        var request = new HttpRequestMessage(HttpMethod.Delete, "/api/v1/alertRules/channels")
        {
            Content = new StringContent(content, Encoding.UTF8, "application/json")
        };

        var response = await Client.SendAsync(request, TestContext.Current.CancellationToken);

        // Assert
        response.EnsureSuccessStatusCode();

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var cache = scope.ServiceProvider.GetRequiredService<IAlertRuleProvider>();
        var channels = await uow.AlertRules.GetAllChannelsAsync(TestContext.Current.CancellationToken);

        Assert.DoesNotContain(channels, c => c.Id == channelId);
        Assert.Equal(beforeCount, GetCachedRuleCount(cache));
        Assert.Equal(string.Empty, await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken));
    }

    [Fact]
    public async Task Delete_NonExistent_AlertChannel_Returns_NotFound()
    {
        await using var beforeScope = Services.CreateAsyncScope();
        var beforeCache = beforeScope.ServiceProvider.GetRequiredService<IAlertRuleProvider>();
        var beforeCount = GetCachedRuleCount(beforeCache);

        // Arrange
        var nonExistentId = Guid.NewGuid();
        var content = $$"""
        {
            "ids": ["{{nonExistentId}}"]
        }
        """;

        // Act
        var request = new HttpRequestMessage(HttpMethod.Delete, "/api/v1/alertRules/channels")
        {
            Content = new StringContent(content, Encoding.UTF8, "application/json")
        };

        var response = await Client.SendAsync(request, TestContext.Current.CancellationToken);

        // Assert
        Assert.Equal(System.Net.HttpStatusCode.NotFound, response.StatusCode);

        await using var afterScope = Services.CreateAsyncScope();
        var afterCache = afterScope.ServiceProvider.GetRequiredService<IAlertRuleProvider>();
        Assert.Equal(beforeCount, GetCachedRuleCount(afterCache));
    }

    private static int GetCachedRuleCount(IAlertRuleProvider cache)
        => cache.Current.ByType.SelectMany(x => x.Value).Count();
}
