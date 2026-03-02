using Domain;
using Application.Services.Alerts;
using Domain.Contracts.Interfaces;
using Domain.Entities.Alerts;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;
using System.Text;

namespace Tests.Integration.Application.Features.Alerters;

public class AlertRulePatchTests : IntegrationTestBase
{
    private Guid ruleId;
    private Guid channelId;
    private Guid channelId2;

    protected override async ValueTask SeedDbAsync(IUnitOfWork uow)
    {
        var rule = new AlertRule(
            type: AlertType.PlatformUnreachable,
            severity: AlertSeverity.Warning,
            cooldownSeconds: 300,
            isEnabled: true,
            createdByActorId: Constants.SystemId);

        await uow.AlertRules.AddAlertRuleAsync(rule, TestContext.Current.CancellationToken);

        var channel = new AlertChannel(
            alertDestination: AlertDestination.Slack,
            url: "https://hooks.slack.com/services/patch",
            isActive: true,
            createdByActorId: Constants.SystemId);
        await uow.AlertRules.AddChannelAsync(channel, TestContext.Current.CancellationToken);

        var channel2 = new AlertChannel(
            alertDestination: AlertDestination.Discord,
            url: "https://discord.com/api/webhooks/patch",
            isActive: true,
            createdByActorId: Constants.SystemId);
        await uow.AlertRules.AddChannelAsync(channel2, TestContext.Current.CancellationToken);

        await uow.CommitAsync(TestContext.Current.CancellationToken);
        ruleId = rule.Id;
        channelId = channel.Id;
        channelId2 = channel2.Id;
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
        var cache = scope.ServiceProvider.GetRequiredService<IAlertRuleProvider>();
        var updated = await uow.AlertRules.GetByIdAsync(ruleId, TestContext.Current.CancellationToken);

        Assert.NotNull(updated);
        Assert.Equal(AlertSeverity.Critical, updated.Severity);
        Assert.Equal(600, updated.CooldownSeconds);
        Assert.False(updated.IsEnabled);
        Assert.Contains(cache.Current.Get(AlertType.PlatformUnreachable), r => r.Id == ruleId && r.Severity == AlertSeverity.Critical);
        await VerifyJson(responseBody);
    }

    [Fact]
    public async Task Patch_AlertRule_With_Invalid_CooldownSeconds_Should_Return_BadRequest()
    {
        await using var beforeScope = Services.CreateAsyncScope();
        var beforeCache = beforeScope.ServiceProvider.GetRequiredService<IAlertRuleProvider>();
        var beforeCount = GetCachedRuleCount(beforeCache);

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

        await using var afterScope = Services.CreateAsyncScope();
        var afterCache = afterScope.ServiceProvider.GetRequiredService<IAlertRuleProvider>();
        Assert.Equal(beforeCount, GetCachedRuleCount(afterCache));
    }

    [Fact]
    public async Task Patch_AlertRule_Should_Update_Channels()
    {
        // Arrange - link first channel
        var linkChannelPatchJson = $$"""
        {
          "channels": ["{{channelId}}"]
        }
        """;
        var linkContent = new StringContent(linkChannelPatchJson, Encoding.UTF8, "application/merge-patch+json");

        // Act - link first channel
        var linkResponse = await Client.PatchAsync($"/api/v1/alerters/rules/{ruleId}", linkContent, cancellationToken: TestContext.Current.CancellationToken);
        var linkBody = await linkResponse.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        Assert.True(linkResponse.IsSuccessStatusCode, linkBody);

        await using (var scope1 = Services.CreateAsyncScope())
        {
            var uow1 = scope1.ServiceProvider.GetRequiredService<IUnitOfWork>();
            var afterFirstPatch = await uow1.AlertRules.GetByIdAsync(ruleId, TestContext.Current.CancellationToken);

            Assert.NotNull(afterFirstPatch);
            Assert.Contains(channelId, afterFirstPatch.Channels);
            Assert.DoesNotContain(channelId2, afterFirstPatch.Channels);
        }

        // Arrange - remove first channel and link second channel
        var switchChannelPatchJson = $$"""
        {
          "channels": ["{{channelId2}}"]
        }
        """;
        var switchContent = new StringContent(switchChannelPatchJson, Encoding.UTF8, "application/merge-patch+json");

        // Act - switch channel link
        var switchResponse = await Client.PatchAsync($"/api/v1/alerters/rules/{ruleId}", switchContent, cancellationToken: TestContext.Current.CancellationToken);
        var switchBody = await switchResponse.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        Assert.True(switchResponse.IsSuccessStatusCode, switchBody);

        // Assert
        await using var scope2 = Services.CreateAsyncScope();
        var uow2 = scope2.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var cache = scope2.ServiceProvider.GetRequiredService<IAlertRuleProvider>();
        var afterSecondPatch = await uow2.AlertRules.GetByIdAsync(ruleId, TestContext.Current.CancellationToken);

        Assert.NotNull(afterSecondPatch);
        Assert.DoesNotContain(channelId, afterSecondPatch.Channels);
        Assert.Contains(channelId2, afterSecondPatch.Channels);
        Assert.Single(afterSecondPatch.Channels);
        Assert.Contains(cache.Current.Get(AlertType.PlatformUnreachable), r => r.Id == ruleId && r.Channels.Contains(channelId2));
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

    [Fact]
    public async Task Patch_AlertChannel_Should_Apply_MergePatch()
    {
        await using var beforeScope = Services.CreateAsyncScope();
        var beforeCache = beforeScope.ServiceProvider.GetRequiredService<IAlertRuleProvider>();
        var beforeCount = GetCachedRuleCount(beforeCache);

        // Arrange
        var patchJson = """
        {
          "url": "https://hooks.slack.com/services/patched",
          "isActive": false
        }
        """;
        var content = new StringContent(patchJson, Encoding.UTF8, "application/merge-patch+json");

        // Act
        var response = await Client.PatchAsync($"/api/v1/alerters/channels/{channelId}", content, cancellationToken: TestContext.Current.CancellationToken);
        response.EnsureSuccessStatusCode();

        // Assert
        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var cache = scope.ServiceProvider.GetRequiredService<IAlertRuleProvider>();
        var updated = await uow.AlertRules.GetChannelByIdAsync(channelId, TestContext.Current.CancellationToken);

        Assert.NotNull(updated);
        Assert.Equal("https://hooks.slack.com/services/patched", updated.Url);
        Assert.False(updated.IsActive);
        Assert.Equal(beforeCount, GetCachedRuleCount(cache));
    }

    [Fact]
    public async Task Patch_NonExistent_AlertChannel_Should_Return_NotFound()
    {
        await using var beforeScope = Services.CreateAsyncScope();
        var beforeCache = beforeScope.ServiceProvider.GetRequiredService<IAlertRuleProvider>();
        var beforeCount = GetCachedRuleCount(beforeCache);

        // Arrange
        var nonExistentId = Guid.NewGuid();
        var patchJson = """
        {
          "isActive": false
        }
        """;
        var content = new StringContent(patchJson, Encoding.UTF8, "application/merge-patch+json");

        // Act
        var response = await Client.PatchAsync($"/api/v1/alerters/channels/{nonExistentId}", content, cancellationToken: TestContext.Current.CancellationToken);

        // Assert
        Assert.Equal(System.Net.HttpStatusCode.NotFound, response.StatusCode);

        await using var afterScope = Services.CreateAsyncScope();
        var afterCache = afterScope.ServiceProvider.GetRequiredService<IAlertRuleProvider>();
        Assert.Equal(beforeCount, GetCachedRuleCount(afterCache));
    }

    private static int GetCachedRuleCount(IAlertRuleProvider cache)
        => cache.Current.ByResourceType.SelectMany(x => x.Value).Count();
}
