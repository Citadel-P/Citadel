using Domain.Contracts.Interfaces;
using Domain;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;
using System.Net.Http.Json;
using System.Text.Json;

namespace Tests.Integration.Application.Features.Alerters;

public class AlertRuleViewTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    [Fact]
    public async Task List_AlertRules_Should_Include_Default_SwarmService_Failure_Rule()
    {
        var response = await Client.GetAsync("/api/v1/alertRules", TestContext.Current.CancellationToken);
        response.EnsureSuccessStatusCode();

        using var document = await JsonDocument.ParseAsync(
            await response.Content.ReadAsStreamAsync(TestContext.Current.CancellationToken),
            cancellationToken: TestContext.Current.CancellationToken);

        var items = document.RootElement.GetProperty("alertRules").EnumerateArray();
        Assert.Contains(items, item =>
            item.GetProperty("type").GetString() == nameof(AlertType.SwarmServiceOperationFailed));
    }

    [Fact]
    public async Task List_AlertRules_Should_Return_Only_Rules_User_Is_Permitted_To_View()
    {
        var visibleRuleId = await CreateAlertRuleAsync("rule-visible");
        await CreateAlertRuleAsync("rule-hidden");

        var subject = await CreateAuthorizationSubjectAsync(
            resourceGrants: [new ResourceGrant(ResourceType.Alert, visibleRuleId, PermissionLevel.Read)]);

        Client.DefaultRequestHeaders.Authorization = new System.Net.Http.Headers.AuthenticationHeaderValue(
            "Bearer",
            CreateJwtToken(subject.UserId, subject.ActorId));

        var response = await Client.GetAsync("/api/v1/alertRules", TestContext.Current.CancellationToken);
        response.EnsureSuccessStatusCode();

        using var document = await JsonDocument.ParseAsync(
            await response.Content.ReadAsStreamAsync(TestContext.Current.CancellationToken),
            cancellationToken: TestContext.Current.CancellationToken);

        var items = document.RootElement.GetProperty("alertRules");
        Assert.Equal(1, items.GetArrayLength());
        Assert.Equal(visibleRuleId, items[0].GetProperty("id").GetGuid());
        Assert.Equal("rule-visible", items[0].GetProperty("name").GetString());
    }

    private async Task<Guid> CreateAlertRuleAsync(string name)
    {
        var payload = new
        {
            Name = name,
            Description = "rule description",
            Type = "PlatformUnreachable",
            Severity = "Warning",
            CooldownSeconds = 60,
            RequiredMatches = (int?)null,
            Threshold = (double?)null,
            ChannelIds = Array.Empty<Guid>(),
            LimitedTo = Array.Empty<object>(),
            QuietHours = Array.Empty<object>()
        };

        var response = await Client.PostAsJsonAsync(
            "/api/v1/alertRules",
            payload,
            cancellationToken: TestContext.Current.CancellationToken);

        response.EnsureSuccessStatusCode();

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        return (await uow.AlertRules.GetAllAsync(TestContext.Current.CancellationToken))
            .Single(x => x.Name == name)
            .Id;
    }
}
